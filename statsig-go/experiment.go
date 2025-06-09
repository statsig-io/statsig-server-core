package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/

type Experiment struct {
	Name               string                 `json:"name"`
	Value              map[string]interface{} `json:"value"`
	RuleID             string                 `json:"rule_id"`
	EvaluationDetails  EvaluationDetails      `json:"details"`
	IdType             string                 `json:"id_type"`
	GroupName          string                 `json:"group_name"`
	SecondaryExposures []SecondaryExposure    `json:"__evaluation"`
}

type SecondaryExposure struct {
	Gate      string `json:"gate"`
	GateValue string `json:"gateValue"`
	RuleID    string `json:"ruleID"`
}

func (e *Experiment) setSecondaryExposures(secondaryExposures []SecondaryExposure) {
	e.SecondaryExposures = secondaryExposures
}
