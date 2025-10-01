package statsig_go_core

type StatsigUser struct {
	ref uint64
}

func NewStatsigUser(userID string) *StatsigUser {
	return &StatsigUser{
		ref: GetFFI().statsig_user_create(userID, "", "", "", "", "", "", "", "", ""),
	}
}
