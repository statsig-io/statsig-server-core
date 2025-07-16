package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
import (
	"runtime"

	"github.com/statsig-io/private-statsig-server-core/statsig-go/utils"
)

// TODO(varshaa): add in remaining options that aren't being passed into statsig_options_create rn
type StatsigOptions struct {
	SpecsUrl                    *string
	LogEventUrl                 *string
	Environment                 *string
	EventLoggingFlushIntervalMs int32
	EventLoggingMaxQueueSize    int32
	SpecsSyncIntervalMs         int32
	OutputLogLevel              *string
	innerRef                    uint64
	DisableCountryLookup        *bool
	DisableUserAgentParsing     *bool
	WaitForCountryLookupInit    *bool
	WaitForUserAgentInit        *bool
	DisableAllLogging           *bool
}

type StatsigOptionsBuilder struct {
	statsigOptions StatsigOptions
}

func ResolveDefault(value *string) *C.char {
	if value != nil {
		return C.CString(*value)
	}
	return nil
}

func (o *StatsigOptionsBuilder) Build() *StatsigOptions {

	optionsRef := C.statsig_options_create(
		ResolveDefault(o.statsigOptions.SpecsUrl),
		ResolveDefault(o.statsigOptions.LogEventUrl),
		C.uint64_t(0),
		C.uint64_t(0),
		ResolveDefault(o.statsigOptions.Environment),
		C.int(o.statsigOptions.EventLoggingFlushIntervalMs),
		C.int(o.statsigOptions.EventLoggingMaxQueueSize),
		C.int(o.statsigOptions.SpecsSyncIntervalMs),
		ResolveDefault(o.statsigOptions.OutputLogLevel),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.DisableCountryLookup)),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.DisableUserAgentParsing)),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.WaitForCountryLookupInit)),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.WaitForUserAgentInit)),
		C.int(utils.ConvertToSafeOptBool(nil)), // enableIDLists, not used in this version
		ResolveDefault(nil),                    // idListsUrl, not used in this version
		C.int(-1),                              // idListsSyncIntervalMs, not used in this version
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.DisableAllLogging)),
		ResolveDefault(nil), // idListsUrl, not used in this version
	)

	o.statsigOptions.innerRef = uint64(optionsRef)

	opt := &o.statsigOptions

	// Set finalizer on the Go object
	runtime.SetFinalizer(opt, func(obj *StatsigOptions) {
		C.statsig_options_release(C.uint64_t(obj.innerRef))

	})

	return opt
}

func NewStatsigOptionsBuilder() *StatsigOptionsBuilder {
	return &StatsigOptionsBuilder{
		statsigOptions: StatsigOptions{
			EventLoggingFlushIntervalMs: -1,
			EventLoggingMaxQueueSize:    -1,
			SpecsSyncIntervalMs:         -1,
			OutputLogLevel:              nil,
		},
	}
}

func (o *StatsigOptionsBuilder) WithSpecsUrl(specsUrl string) *StatsigOptionsBuilder {
	o.statsigOptions.SpecsUrl = &specsUrl
	return o
}

func (o *StatsigOptionsBuilder) WithLogEventUrl(logEventUrl string) *StatsigOptionsBuilder {
	o.statsigOptions.LogEventUrl = &logEventUrl
	return o
}

func (o *StatsigOptionsBuilder) WithEnvironment(env string) *StatsigOptionsBuilder {
	o.statsigOptions.Environment = &env
	return o
}

func (o *StatsigOptionsBuilder) WithEventLoggingFlushIntervalMs(interval int32) *StatsigOptionsBuilder {
	o.statsigOptions.EventLoggingFlushIntervalMs = interval
	return o
}

func (o *StatsigOptionsBuilder) WithEventLoggingMaxQueueSize(eventLoggingMaxQueueSize int32) *StatsigOptionsBuilder {
	o.statsigOptions.EventLoggingMaxQueueSize = eventLoggingMaxQueueSize
	return o
}

func (o *StatsigOptionsBuilder) WithSpecsSyncIntervalMs(specsSyncIntervalMs int32) *StatsigOptionsBuilder {
	o.statsigOptions.SpecsSyncIntervalMs = specsSyncIntervalMs
	return o
}

func (o *StatsigOptionsBuilder) WithOutputLogLevel(outputLogLevel string) *StatsigOptionsBuilder {
	o.statsigOptions.OutputLogLevel = &outputLogLevel
	return o
}

func (o *StatsigOptionsBuilder) WithDisableCountryLookup(value bool) *StatsigOptionsBuilder {
	o.statsigOptions.DisableCountryLookup = &value
	return o
}

func (o *StatsigOptionsBuilder) WithDisableUserAgentParsing(value bool) *StatsigOptionsBuilder {
	o.statsigOptions.DisableUserAgentParsing = &value
	return o
}

func (o *StatsigOptionsBuilder) WithWaitForCountryLookupInit(value bool) *StatsigOptionsBuilder {
	o.statsigOptions.WaitForCountryLookupInit = &value
	return o
}

func (o *StatsigOptionsBuilder) WithWaitForUserAgentInit(value bool) *StatsigOptionsBuilder {
	o.statsigOptions.WaitForUserAgentInit = &value
	return o
}

func (o *StatsigOptionsBuilder) WithDisableAllLogging(value bool) *StatsigOptionsBuilder {
	o.statsigOptions.DisableAllLogging = &value
	return o
}
