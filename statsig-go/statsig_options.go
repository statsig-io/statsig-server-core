package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
import (
	"runtime"
)

// TODO(varshaa): add in remaining options that aren't being passed into statsig_options_create rn
type StatsigOptions struct {
	SpecsUrl                    *string
	LogEventUrl                 *string
	SpecsAdapterRef             *string
	EventLoggingAdapterRef      *string
	Environment                 *string
	EventLoggingFlushIntervalMs int32
	EventLoggingMaxQueueSize    int32
	SpecsSyncIntervalMs         int32
	OutputLogLevel              *string
	innerRef                    *C.char
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
		ResolveDefault(o.statsigOptions.SpecsAdapterRef),
		ResolveDefault(o.statsigOptions.EventLoggingAdapterRef),
		ResolveDefault(o.statsigOptions.Environment),
		C.int(o.statsigOptions.EventLoggingFlushIntervalMs),
		C.int(o.statsigOptions.EventLoggingMaxQueueSize),
		C.int(o.statsigOptions.SpecsSyncIntervalMs),
		ResolveDefault(o.statsigOptions.OutputLogLevel),
	)

	o.statsigOptions.innerRef = optionsRef

	opt := &o.statsigOptions

	// Set finalizer on the Go object
	runtime.SetFinalizer(opt, func(obj *StatsigOptions) {
		C.statsig_options_release(obj.innerRef)

	})

	return opt
}

func NewStatsigOptionsBuilder() *StatsigOptionsBuilder {
	return &StatsigOptionsBuilder{
		statsigOptions: StatsigOptions{
			SpecsAdapterRef:             nil,
			EventLoggingAdapterRef:      nil,
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

func (o *StatsigOptionsBuilder) WithSpecsAdapterRef(specsAdapterRef string) *StatsigOptionsBuilder {
	o.statsigOptions.SpecsAdapterRef = &specsAdapterRef
	return o
}

func (o *StatsigOptionsBuilder) WithEventLoggingAdapterRef(eventLoggingAdapterRef string) *StatsigOptionsBuilder {
	o.statsigOptions.EventLoggingAdapterRef = &eventLoggingAdapterRef
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
