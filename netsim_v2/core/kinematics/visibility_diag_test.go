package kinematics

import (
	"math"
	"testing"
)

// capAreaFraction returns the fraction of Earth's surface from which a
// satellite at altitudeKm is visible above minElevDeg. This is the analytic
// expectation for the per-(satellite, time) visible-sample fraction when
// ground tracks uniformly sample the sphere.
func capAreaFraction(altitudeKm, minElevDeg float64) float64 {
	Re := earthRadiusKm
	rs := Re + altitudeKm
	e := minElevDeg * math.Pi / 180
	// Slant range at elevation e: rho = -Re*sin(e) + sqrt(rs^2 - Re^2*cos^2(e))
	rho := -Re*math.Sin(e) + math.Sqrt(rs*rs-Re*Re*math.Cos(e)*math.Cos(e))
	// Central angle between ground station and satellite subpoint.
	cosGamma := (Re + rho*math.Sin(e)) / rs
	return (1 - cosGamma) / 2
}

// TestVisibilityPhysics checks two invariants over one full orbital period:
//
//  1. Per-satellite visible fraction ≈ spherical-cap area fraction (the
//     geometry engine is unbiased). This is the number link_stats.visible_pct
//     aggregates over (satellite, time) samples, so its small magnitude
//     (~1-2%) is expected physics, not a bug.
//
//  2. Reports the constellation-level any-visible fraction (>=1 satellite in
//     view), which is the operationally meaningful coverage number.
func TestVisibilityPhysics(t *testing.T) {
	const (
		altKm     = 550.0
		inclDeg   = 53.0
		minElev   = 10.0
		periodSec = 5730.0 // ~one orbit at 550 km
		stepSec   = 2.0
	)

	gs := &GroundStation{LatDeg: 37.77, LonDeg: -122.42} // San Francisco

	// Walker-like: 4 planes x 6 sats, RAAN spread, Walker-delta phasing.
	sats := make([]*Satellite, 0, 24)
	for p := 0; p < 4; p++ {
		for k := 0; k < 6; k++ {
			raan := 90.0 * float64(p)
			phase := 2*math.Pi*float64(k)/6 + math.Pi*float64(p)/12
			sats = append(sats, NewSatellite(p*6+k, altKm, inclDeg*math.Pi/180, raan*math.Pi/180, phase))
		}
	}

	anyVis := 0
	perSatVis := make([]float64, len(sats))
	nSteps := 0.0
	for t := 0.0; t <= periodSec; t += stepSec {
		nSteps++
		any := false
		for i, s := range sats {
			if gs.IsVisible(s, minElev, t) {
				perSatVis[i]++
				any = true
			}
		}
		if any {
			anyVis++
		}
	}

	expected := capAreaFraction(altKm, minElev)
	avgPerSat := 0.0
	for _, v := range perSatVis {
		avgPerSat += v / nSteps
	}
	avgPerSat /= float64(len(sats))
	anyPct := float64(anyVis) / nSteps * 100

	t.Logf("spherical-cap expectation (h=550km, elev>=10deg): %.3f%%", expected*100)
	t.Logf("measured avg per-satellite visible fraction:      %.3f%%", avgPerSat*100)
	t.Logf("constellation any-visible fraction (>=1 sat):     %.1f%% of the orbit", anyPct)

	// The geometry is correct iff the per-satellite fraction matches the cap
	// area within a loose tolerance (ground tracks at 53 deg inclination from
	// a mid-latitude station are not perfectly uniform).
	if math.Abs(avgPerSat-expected) > 0.6*expected {
		t.Errorf("per-satellite visible fraction %.3f%% deviates from cap expectation %.3f%% by more than 60%%",
			avgPerSat*100, expected*100)
	}
}
