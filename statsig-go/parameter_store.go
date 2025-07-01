package statsig

type ParameterStore struct {
	Name                   string
	Details                map[string]interface{} `json:"details"`
	Options                ParameterStoreOptions  `json:"options"`
	statsigInstance        Statsig
	user                   StatsigUser
	disableExposureLogging bool
}

func (ps *ParameterStore) setStatsigUser(u *StatsigUser) {
	ps.user = *u
}

func (ps *ParameterStore) setStatsigInstance(s *Statsig) {
	ps.statsigInstance = *s
}

func (ps *ParameterStore) setDisableExposureLogging(disableExposureLogging bool) {
	ps.disableExposureLogging = disableExposureLogging
}

func (ps *ParameterStore) GetString(parameterName string, defaultValue string) string {
	return ps.statsigInstance.GetStringFromParameterStore(ps.user, ps.Name, parameterName, defaultValue, &ps.Options)
}

func (ps *ParameterStore) GetBoolean(parameterName string, defaultValue bool) bool {
	return ps.statsigInstance.GetBooleanFromParameterStore(ps.user, ps.Name, parameterName, defaultValue, &ps.Options)
}

func (ps *ParameterStore) GetFloat64(parameterName string, defaultValue float64) float64 {
	return ps.statsigInstance.GetFloat64FromParameterStore(ps.user, ps.Name, parameterName, defaultValue, &ps.Options)
}

func (ps *ParameterStore) GetInt(parameterName string, defaultValue int) int {
	return ps.statsigInstance.GetIntFromParameterStore(ps.user, ps.Name, parameterName, defaultValue, &ps.Options)
}

func (ps *ParameterStore) GetInt64(parameterName string, defaultValue int64) int64 {
	return ps.statsigInstance.GetInt64FromParameterStore(ps.user, ps.Name, parameterName, defaultValue, &ps.Options)
}

func (ps *ParameterStore) GetInterface(parameterName string, defaultValue []interface{}) []interface{} {
	return ps.statsigInstance.GetInterfaceFromParameterStore(ps.user, ps.Name, parameterName, defaultValue, &ps.Options)
}

func (ps *ParameterStore) GetMap(parameterName string, defaultValue map[string]interface{}) map[string]interface{} {
	return ps.statsigInstance.GetMapFromParameterStore(ps.user, ps.Name, parameterName, defaultValue, &ps.Options)
}
