package main

import (
	"math"
)

// Physical constants (WGS-84).
const (
	earthRadiusKm = 6371.0
	muKm3S2       = 398600.4418
	siderealSec   = 86164.09 // Earth rotation period about its axis (sideral day)
	speedOfLight  = 299792.458 // km/s
)

// Vec3 is a 3-vector in ECEF coordinates (kilometers).
type Vec3 struct {
	X, Y, Z float64
}

func (v Vec3) norm() float64 {
	return math.Sqrt(v.X*v.X + v.Y*v.Y + v.Z*v.Z)
}

func (v Vec3) dot(o Vec3) float64 {
	return v.X*o.X + v.Y*o.Y + v.Z*o.Z
}

func (v Vec3) sub(o Vec3) Vec3 {
	return Vec3{v.X - o.X, v.Y - o.Y, v.Z - o.Z}
}

// Satellite models a satellite on a circular LEO orbit. The orbit is fully
// described by inclination, RAAN, and an initial argument of latitude (phase);
// position over time is deterministic and seeded, so runs are reproducible.
type Satellite struct {
	ID          int
	Inclination float64 // radians
	RAAN        float64 // radians, right ascension of ascending node
	Phase       float64 // radians, initial argument of latitude
	AltitudeKm  float64
	PeriodSec   float64
}

// NewSatellite places satellite id on a circular orbit at altitudeKm with the
// given inclination, RAAN, and initial phase.
func NewSatellite(id int, altitudeKm, inclination, raan, phase float64) *Satellite {
	radius := earthRadiusKm + altitudeKm
	period := 2 * math.Pi * math.Sqrt(math.Pow(radius, 3)/muKm3S2)
	return &Satellite{
		ID:          id,
		Inclination: inclination,
		RAAN:        raan,
		Phase:       phase,
		AltitudeKm:  altitudeKm,
		PeriodSec:   period,
	}
}

// PositionECEF returns the satellite ECEF position at elapsed time t (seconds
// since the simulation epoch). Earth rotation is folded in so the result is in
// the ground-station-fixed frame.
func (s *Satellite) PositionECEF(t float64) Vec3 {
	u := s.Phase + 2*math.Pi*t/s.PeriodSec // argument of latitude
	radius := earthRadiusKm + s.AltitudeKm

	// Position in the orbital plane, then rotate by inclination and RAAN,
	// then subtract the Earth's rotation to land in ECEF.
	xp := radius * math.Cos(u)
	yp := radius * math.Sin(u)
	zp := 0.0

	// Rotate by inclination about the x-axis.
	ci := math.Cos(s.Inclination)
	si := math.Sin(s.Inclination)
	y1 := yp*ci - zp*si
	z1 := yp*si + zp*ci

	// Rotate by RAAN about the z-axis (inertial frame).
	cr := math.Cos(s.RAAN)
	sr := math.Sin(s.RAAN)
	x2 := xp*cr - y1*sr
	y2 := xp*sr + y1*cr
	z2 := z1

	// Fold in Earth rotation (angle since epoch), producing ECEF.
	theta := 2 * math.Pi * t / siderealSec
	ct := math.Cos(theta)
	st := math.Sin(theta)
	x := x2*ct + y2*st
	y := -x2*st + y2*ct
	z := z2
	return Vec3{X: x, Y: y, Z: z}
}

// GroundStation is a fixed observer on the Earth's surface.
type GroundStation struct {
	LatDeg, LonDeg float64
	AltKm          float64
}

// PositionECEF returns the ground station ECEF position.
func (g *GroundStation) PositionECEF() Vec3 {
	lat := g.LatDeg * math.Pi / 180
	lon := g.LonDeg * math.Pi / 180
	r := earthRadiusKm + g.AltKm
	cl := math.Cos(lat)
	return Vec3{
		X: r * cl * math.Cos(lon),
		Y: r * cl * math.Sin(lon),
		Z: r * math.Sin(lat),
	}
}

// ElevationDeg returns the satellite elevation angle above the local horizon
// as seen from the ground station, in degrees. Negative means below horizon.
func (g *GroundStation) ElevationDeg(s *Satellite, t float64) float64 {
	gs := g.PositionECEF()
	sat := s.PositionECEF(t)
	r := sat.sub(gs)
	if r.norm() == 0 {
		return 90
	}
	// Local vertical (up) at the ground station.
	up := Vec3{gs.X, gs.Y, gs.Z}
	up = Vec3{X: up.X / up.norm(), Y: up.Y / up.norm(), Z: up.Z / up.norm()}
	cosZenith := r.dot(up) / r.norm()
	cosZenith = clamp(cosZenith, -1, 1)
	return 90 - math.Acos(cosZenith)*180/math.Pi
}

// SlantRangeKm returns the straight-line distance from ground station to
// satellite at time t.
func (g *GroundStation) SlantRangeKm(s *Satellite, t float64) float64 {
	gs := g.PositionECEF()
	return gs.sub(s.PositionECEF(t)).norm()
}

// OneWayLatencySec returns the free-space propagation delay for a signal
// traversing the link at time t.
func (g *GroundStation) OneWayLatencySec(s *Satellite, t float64) float64 {
	return g.SlantRangeKm(s, t) / speedOfLight
}

// IsVisible reports whether the satellite is above the minimum elevation at t.
func (g *GroundStation) IsVisible(s *Satellite, minElevDeg, t float64) bool {
	return g.ElevationDeg(s, t) >= minElevDeg
}

func clamp(x, lo, hi float64) float64 {
	if x < lo {
		return lo
	}
	if x > hi {
		return hi
	}
	return x
}

// NewOrbitRing creates count satellites spread uniformly in phase around a
// single orbital plane (same inclination and RAAN), plus a second plane if
// count is large enough. Returns them in ID order.
func NewOrbitRing(count int, altitudeKm, inclinationDeg, raanDeg float64) []*Satellite {
	sats := make([]*Satellite, count)
	for i := 0; i < count; i++ {
		phase := 2 * math.Pi * float64(i) / float64(count)
		sats[i] = NewSatellite(i, altitudeKm, inclinationDeg*math.Pi/180, raanDeg*math.Pi/180, phase)
	}
	return sats
}
