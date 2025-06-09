package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import (
	"C"
)

type Layer struct {
	Name                    string                 `json:"name"`
	Value                   map[string]interface{} `json:"__value"`
	RuleID                  string                 `json:"rule_id"`
	EvaluationDetails       EvaluationDetails      `json:"details"`
	IdType                  string                 `json:"id_type"`
	GroupName               string                 `json:"group_name"`
	AllocatedExperimentName string                 `json:"allocated_experiment_name"`
	statsigInstance         Statsig
	disableExposureLogging  bool
}

func (l *Layer) setStatsigInstance(statsig *Statsig) {
	l.statsigInstance = *statsig
}

func (l *Layer) setDisableExposureLogging(disableExposureLogging bool) {
	l.disableExposureLogging = disableExposureLogging
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
		l.manuallyLogLayerParamExposure(u, paramName)
	}
	return val
}

func (l *Layer) manuallyLogLayerParamExposure(u *StatsigUser, paramName string) {
	C.statsig_manually_log_layer_parameter_exposure(
		C.ulonglong(l.statsigInstance.InnerRef),
		C.ulonglong(u.innerRef),
		C.CString(l.Name),
		C.CString(paramName),
	)
}
