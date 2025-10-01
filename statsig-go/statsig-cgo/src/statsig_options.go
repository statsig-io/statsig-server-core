package statsig

/*
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
import (
	"runtime"

	"github.com/statsig-io/statsig-server-core/statsig-go/src/utils"
)

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
	WaitForCountryLookupInit    *bool
	WaitForUserAgentInit        *bool
	EnableIdLists               *bool
	IdListsUrl                  *string
	IdListsSyncIntervalMs       int32
	DisableAllLogging           *bool
	DisableNetwork              *bool
	GlobalCustomFields          *string
	ObservabilityClientRef      uint64
	DataStoreRef                uint64
	InitTimeoutMs               int32
	FallbackToStatsigApi        *bool
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
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.WaitForCountryLookupInit)),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.WaitForUserAgentInit)),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.EnableIdLists)),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.DisableNetwork)),
		ResolveDefault(o.statsigOptions.IdListsUrl),
		C.int(o.statsigOptions.IdListsSyncIntervalMs),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.DisableAllLogging)),
		ResolveDefault(o.statsigOptions.GlobalCustomFields),
		C.uint64_t(o.statsigOptions.ObservabilityClientRef),
		C.uint64_t(o.statsigOptions.DataStoreRef),
		C.int(o.statsigOptions.InitTimeoutMs),
		C.int(utils.ConvertToSafeOptBool(o.statsigOptions.FallbackToStatsigApi)),
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
			IdListsSyncIntervalMs:       -1,
			InitTimeoutMs:               -1,
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

func (o *StatsigOptionsBuilder) WithDisableNetwork(value bool) *StatsigOptionsBuilder {
	o.statsigOptions.DisableNetwork = &value
	return o
}

func (o *StatsigOptionsBuilder) WithEnableIdLists(value bool) *StatsigOptionsBuilder {
	o.statsigOptions.EnableIdLists = &value
	return o
}

func (o *StatsigOptionsBuilder) WithIdListsUrl(value string) *StatsigOptionsBuilder {
	o.statsigOptions.IdListsUrl = &value
	return o
}

func (o *StatsigOptionsBuilder) WithIDListsSyncIntervalMs(value int32) *StatsigOptionsBuilder {
	o.statsigOptions.IdListsSyncIntervalMs = value
	return o
}

func (o *StatsigOptionsBuilder) WithGlobalCustomFields(value string) *StatsigOptionsBuilder {
	o.statsigOptions.GlobalCustomFields = &value
	return o
}

func (o *StatsigOptionsBuilder) WithDataStore(dataStore DataStoreInterface) *StatsigOptionsBuilder {
	o.statsigOptions.DataStoreRef = NewDataStore(dataStore)
	return o
}

func (o *StatsigOptionsBuilder) WithObservabilityClient(observabilityClient ObservabilityClientInterface) *StatsigOptionsBuilder {
	o.statsigOptions.ObservabilityClientRef = NewObservabilityClient(observabilityClient)
	return o
}

func (o *StatsigOptionsBuilder) WithInitTimeoutMs(value int32) *StatsigOptionsBuilder {
	o.statsigOptions.InitTimeoutMs = value
	return o
}

func (o *StatsigOptionsBuilder) WithFallbackToStatsigApi(value bool) *StatsigOptionsBuilder {
	o.statsigOptions.FallbackToStatsigApi = &value
	return o
}
