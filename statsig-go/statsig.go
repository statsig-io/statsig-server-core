package statsig_go_core

type Statsig struct {
	ref uint64
}

func NewStatsig(sdkKey string) *Statsig {
	return &Statsig{
		ref: GetFFI().statsig_create(sdkKey, 0),
	}
}

func (s *Statsig) Initialize() {
	GetFFI().statsig_initialize_blocking(s.ref)
}

func (s *Statsig) Shutdown() {
	GetFFI().statsig_shutdown_blocking(s.ref)
}

func (s *Statsig) FlushEvents() {
	GetFFI().statsig_flush_events_blocking(s.ref)
}

func (s *Statsig) CheckGate(user *StatsigUser, gateName string) bool {
	return GetFFI().statsig_check_gate(s.ref, user.ref, gateName, "{}")
}
