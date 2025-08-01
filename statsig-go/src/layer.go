package statsig

/*
#cgo CFLAGS: -I../../statsig-ffi/include
#include "statsig_ffi.h"

#include <stdlib.h>
*/
import (
	"C"
)
import "github.com/statsig-io/statsig-server-core/statsig-go/src/utils"

type Layer struct {
	Name                    string                 `json:"name"`
	Value                   map[string]interface{} `json:"__value"`
	RuleID                  string                 `json:"rule_id"`
	EvaluationDetails       EvaluationDetails      `json:"details"`
	IdType                  string                 `json:"id_type"`
	GroupName               string                 `json:"group_name"`
	AllocatedExperimentName string                 `json:"allocated_experiment_name"`
	RawJSON                 string                 `json:"-"`
	statsigInstance         Statsig
	disableExposureLogging  bool
}

func (l *Layer) setStatsigInstance(statsig *Statsig) {
	l.statsigInstance = *statsig
}

func (l *Layer) setDisableExposureLogging(disableExposureLogging bool) {
	l.disableExposureLogging = disableExposureLogging
}

func (l *Layer) setRawResult(json string) {
	l.RawJSON = json
}

func (l *Layer) Get(u *StatsigUser, paramName string) interface{} {
	return l.getValueImpl(paramName, u)
}

func (l *Layer) getValueImpl(paramName string, u *StatsigUser) interface{} {
	val, ok := l.Value[paramName]

	if !ok {
		return ""
	}

	if !l.disableExposureLogging {
		l.logLayerParamExposure(paramName)
	}
	return val
}

func (l *Layer) logLayerParamExposure(paramName string) {
	C.statsig_log_layer_param_exposure(
		C.uint64_t(l.statsigInstance.InnerRef),
		C.CString(l.RawJSON),
		C.CString(paramName),
	)
}
func (l *Layer) GetString(key string, fallback string) string {
	return utils.GetTypedValue(l.Value, key, fallback)
}

func (l *Layer) GetNumber(key string, fallback float64) float64 {
	return utils.GetTypedValue(l.Value, key, fallback)
}

func (l *Layer) GetBool(key string, fallback bool) bool {
	return utils.GetTypedValue(l.Value, key, fallback)
}

func (l *Layer) GetSlice(key string, fallback []interface{}) []interface{} {
	return utils.GetTypedValue(l.Value, key, fallback)
}

func (l *Layer) GetMap(key string, fallback map[string]interface{}) map[string]interface{} {
	return utils.GetTypedValue(l.Value, key, fallback)
}
