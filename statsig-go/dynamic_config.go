package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/

type DynamicConfig struct {
	Name              string                 `json:"name"`
	Value             map[string]interface{} `json:"value"`
	RuleID            string                 `json:"rule_id"`
	EvaluationDetails EvaluationDetails      `json:"details"`
	IdType            string                 `json:"id_type"`
}
