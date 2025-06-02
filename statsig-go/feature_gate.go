package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/

type FeatureGate struct {
	Name              string            `json:"name"`
	Value             bool              `json:"value"`
	RuleID            string            `json:"rule_id"`
	EvaluationDetails EvaluationDetails `json:"details"`
	IdType            string            `json:"id_type"`
}
