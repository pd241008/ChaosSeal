package main

import (
	"math"
	"testing"
)

func TestSatellitePeriod(t *testing.T) {
	s := NewSatellite(0, 550, 0, 0, 0)
	// Circular orbit period at 550 km altitude: T = 2π sqrt(r³/μ).
	want := 2 * math.Pi * math.Sqrt(math.Pow(earthRadiusKm+550, 3)/muKm3S2)
	if math.Abs(s.PeriodSec-want) > 1e-6 {
		t.Errorf("period = %v, want %v", s.PeriodSec, want)
	}
	if s.PeriodSec < 5000 || s.PeriodSec > 6200 {
		t.Errorf("LEO period %v s out of plausible range [5000,6200]", s.PeriodSec)
	}
}

func TestElevationOverhead(t *testing.T) {
	// Equatorial satellite, RAAN 0, phase 0: sub-satellite point is (0°N, 0°E)
	// at t=0, i.e. directly above a ground station at the origin.
	gs := &GroundStation{LatDeg: 0, LonDeg: 0}
	s := NewSatellite(0, 550, 0, 0, 0)

	elev := gs.ElevationDeg(s, 0)
	if elev < 89 {
		t.Errorf("overhead elevation = %v, want near 90", elev)
	}
	slant := gs.SlantRangeKm(s, 0)
	if math.Abs(slant-550) > 10 {
		t.Errorf("overhead slant range = %v km, want ~550", slant)
	}
	lat := gs.OneWayLatencySec(s, 0) * 1000
	if lat < 1 || lat > 3 {
		t.Errorf("overhead one-way latency = %v ms, want ~1.8ms", lat)
	}
}

func TestElevationBelowHorizon(t *testing.T) {
	gs := &GroundStation{LatDeg: 0, LonDeg: 0}
	s := NewSatellite(0, 550, 0, 0, 0)
	// Half a period later the satellite is on the far side of the Earth.
	elev := gs.ElevationDeg(s, s.PeriodSec/2)
	if elev > -80 {
		t.Errorf("opposite-side elevation = %v, want far below horizon", elev)
	}
}

func TestVisibilityRespectsMinElevation(t *testing.T) {
	gs := &GroundStation{LatDeg: 0, LonDeg: 0}
	s := NewSatellite(0, 550, 0, 0, 0)
	if !gs.IsVisible(s, 10, 0) {
		t.Error("overhead satellite should be visible at min elevation 10°")
	}
	if gs.IsVisible(s, 95, 0) {
		t.Error("satellite below 95° elevation should not be visible")
	}
}

func TestNewOrbitRing(t *testing.T) {
	sats := NewOrbitRing(6, 550, 53, 0)
	if len(sats) != 6 {
		t.Fatalf("ring length = %d, want 6", len(sats))
	}
	for i, s := range sats {
		if s.ID != i {
			t.Errorf("satellite ID = %d, want %d", s.ID, i)
		}
		if s.AltitudeKm != 550 {
			t.Errorf("satellite altitude = %v, want 550", s.AltitudeKm)
		}
	}
}
