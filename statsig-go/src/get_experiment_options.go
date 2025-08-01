package statsig

/*
#cgo CFLAGS: -I../../statsig-ffi/include
#include "statsig_ffi.h"

#include <stdlib.h>
*/

type GetExperimentOptions struct {
	DisableExposureLogging bool `json:"disable_exposure_logging"`
}
