package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"errors"
	"runtime"

	"github.com/statsig-io/private-statsig-server-core/statsig-go/utils"
)

type Statsig struct {
	InnerRef uint64
}
type InitializeWithDetails struct {
	Duration          float64         `json:"duration"`
	InitSuccess       bool            `json:"init_success"`
	IsConfigSpecReady bool            `json:"is_config_spec_ready"`
	IsIdListReady     *bool           `json:"is_id_list_ready"`
	Source            string          `json:"source"`
	FailureDetails    *FailureDetails `json:"failure_details"`
}

type FailureDetails struct {
	Reason string                 `json:"reason"`
	Error  map[string]interface{} `json:"error"`
}

func NewStatsig(key string, options StatsigOptions) (*Statsig, error) {
	statsigRef := C.statsig_create(C.CString(key), C.ulonglong(options.innerRef))

	if statsigRef == 0 {
		return nil, errors.New("error occurred in creating Statsig instance")
	}

	s := &Statsig{
		InnerRef: uint64(statsigRef),
	}

	runtime.SetFinalizer(s, func(obj *Statsig) {
		C.statsig_release(C.ulonglong(obj.InnerRef))
	})

	return s, nil

}

func (s *Statsig) Initialize() (bool, error) {
	C.statsig_initialize_blocking(C.ulonglong(s.InnerRef))

	// TODO: return false if statsig_initialize_blocking is updated to return a value
	return true, nil

}

func (s *Statsig) InitializeWithDetails() (InitializeWithDetails, error) {

	res := C.statsig_initialize_with_details(C.ulonglong(s.InnerRef))

	var details InitializeWithDetails

	if res != nil {
		res_str := C.GoString(res)
		err := json.Unmarshal([]byte(res_str), &details)
		if err != nil {
			return InitializeWithDetails{}, err
		}
	}

	return details, nil

}

func (s *Statsig) Shutdown() {
	C.statsig_shutdown_blocking(C.ulonglong(s.InnerRef))

}

func (s *Statsig) GetFeatureGate(user StatsigUser, gateName string, featureGateOptions *CheckGateOptions) FeatureGate {

	var featureGate FeatureGate

	featureGateJson := C.statsig_get_feature_gate(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(gateName), C.CString(utils.ConvertDataToJson(featureGateOptions)))

	if featureGateJson != nil {
		err := json.Unmarshal([]byte(C.GoString(featureGateJson)), &featureGate)
		if err != nil {
			return FeatureGate{}
		}
	}

	return featureGate

}

func (s *Statsig) CheckGate(user StatsigUser, gateName string, gateOptions *CheckGateOptions) bool {

	checkGate := C.statsig_check_gate(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(gateName), C.CString(utils.ConvertDataToJson(gateOptions)))
	return bool(checkGate)
}
