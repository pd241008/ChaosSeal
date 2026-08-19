package crypto

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"io"
	"math/big"
	"net"
	"sync/atomic"
	"time"
)

// TLSBaselineResult captures one real TLS 1.3 handshake measured over the
// simulated link.
type TLSBaselineResult struct {
	HandshakeSec  float64 `json:"handshake_sec"`
	BytesSent     int64   `json:"bytes_sent"`
	BytesReceived int64   `json:"bytes_received"`
	CipherSuite   string  `json:"cipher_suite"`
	AppPayloadSec float64 `json:"app_payload_sec"`
}

// LatencyProvider returns the current one-way link latency.
type LatencyProvider func() time.Duration

// linkConn wraps a net.Conn, adding one-way propagation latency on each write
// and counting bytes in/out.
type linkConn struct {
	net.Conn
	latency LatencyProvider
	bytesIn  atomic.Int64
	bytesOut atomic.Int64
}

func (c *linkConn) Write(p []byte) (int, error) {
	n, err := c.Conn.Write(p)
	c.bytesOut.Add(int64(n))
	if d := c.latency(); d > 0 {
		time.Sleep(d)
	}
	return n, err
}

func (c *linkConn) Read(p []byte) (int, error) {
	n, err := c.Conn.Read(p)
	c.bytesIn.Add(int64(n))
	return n, err
}

// RunTLS13Baseline performs a genuine crypto/tls 1.3 handshake over a
// loopback connection wrapped in the simulated link (latency applied per
// write, bytes counted). It returns handshake timing and byte counts.
func RunTLS13Baseline(latency LatencyProvider, payload []byte) (*TLSBaselineResult, error) {
	cert, err := selfSignedCert()
	if err != nil {
		return nil, err
	}

	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return nil, err
	}
	defer ln.Close()

	type serverResult struct {
		handshake time.Duration
		payload   time.Duration
		err       error
	}
	serverCh := make(chan serverResult, 1)

	go func() {
		conn, err := ln.Accept()
		if err != nil {
			serverCh <- serverResult{err: err}
			return
		}
		defer conn.Close()
		lc := &linkConn{Conn: conn, latency: latency}

		tlsConn := tls.Server(lc, &tls.Config{Certificates: []tls.Certificate{cert}})
		hsStart := time.Now()
		if err := tlsConn.Handshake(); err != nil {
			serverCh <- serverResult{err: err}
			return
		}
		hs := time.Since(hsStart)

		// Echo the payload back; measures application-phase throughput too.
		payloadStart := time.Now()
		if _, err := io.Copy(tlsConn, tlsConn); err != nil {
			serverCh <- serverResult{err: err}
			return
		}
		serverCh <- serverResult{handshake: hs, payload: time.Since(payloadStart)}
	}()

	conn, err := net.Dial("tcp", ln.Addr().String())
	if err != nil {
		return nil, err
	}
	defer conn.Close()
	lc := &linkConn{Conn: conn, latency: latency}

	cfg := &tls.Config{InsecureSkipVerify: true, MinVersion: tls.VersionTLS13, MaxVersion: tls.VersionTLS13}
	client := tls.Client(lc, cfg)
	hsStart := time.Now()
	if err := client.Handshake(); err != nil {
		return nil, err
	}
	hs := time.Since(hsStart)

	if _, err := client.Write(payload); err != nil {
		return nil, err
	}
	echo := make([]byte, len(payload))
	if _, err := io.ReadFull(client, echo); err != nil {
		return nil, err
	}
	appSec := time.Since(hsStart).Seconds()

	client.CloseWrite()
	payloadSec := time.Since(hsStart).Seconds() - hs.Seconds()

	sr := <-serverCh
	if sr.err != nil {
		return nil, sr.err
	}
	_ = appSec

	return &TLSBaselineResult{
		HandshakeSec:  hs.Seconds(),
		BytesSent:     lc.bytesOut.Load(),
		BytesReceived: lc.bytesIn.Load(),
		CipherSuite:   tls.CipherSuiteName(client.ConnectionState().CipherSuite),
		AppPayloadSec: payloadSec,
	}, nil
}

// selfSignedCert generates a throwaway ECDSA P-256 self-signed certificate,
// sufficient for a localhost-only TLS 1.3 handshake.
func selfSignedCert() (tls.Certificate, error) {
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return tls.Certificate{}, err
	}
	tmpl := x509.Certificate{
		SerialNumber: big.NewInt(1),
		Subject:      pkix.Name{CommonName: "chaosseal-netsim"},
		NotBefore:    time.Now().Add(-time.Hour),
		NotAfter:     time.Now().Add(time.Hour),
		KeyUsage:     x509.KeyUsageDigitalSignature | x509.KeyUsageKeyEncipherment,
		ExtKeyUsage:  []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		DNSNames:     []string{"localhost"},
	}
	der, err := x509.CreateCertificate(rand.Reader, &tmpl, &tmpl, &key.PublicKey, key)
	if err != nil {
		return tls.Certificate{}, err
	}
	certPEM := pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: der})
	keyDER, err := x509.MarshalECPrivateKey(key)
	if err != nil {
		return tls.Certificate{}, err
	}
	keyPEM := pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: keyDER})
	return tls.X509KeyPair(certPEM, keyPEM)
}
