package statsig_go_core

import (
	"encoding/json"
	"fmt"
	"runtime"

	"github.com/statsig-io/statsig-go-core/internal"
)

type obsClientArgs struct {
	Metric string            `json:"metric"`
	Value  float64           `json:"value"`
	Tags   map[string]string `json:"tags"`
}

type obsClientErrorArgs struct {
	Tag   string `json:"tag"`
	Error string `json:"error"`
}

type ObservabilityClientFunctions struct {
	Init                                  func()
	Increment                             func(metric string, value float64, tags map[string]string)
	Gauge                                 func(metric string, value float64, tags map[string]string)
	Dist                                  func(metric string, value float64, tags map[string]string)
	Error                                 func(tag string, err string)
	ShouldEnableHighCardinalityForThisTag func(tag string) bool
}

type ObservabilityClient struct {
	functions ObservabilityClientFunctions
	ref       uint64
}

func NewObservabilityClient(functions ObservabilityClientFunctions) *ObservabilityClient {
	client := &ObservabilityClient{
		functions: functions,
		ref:       0,
	}

	client.ref = GetFFI().observability_client_create(
		client.functions.Init,
		// Increment
		func(argsPtr *byte, argsLength uint64) {
			data, err := tryMarshalStandardArgs(argsPtr, argsLength)
			if err != nil {
				fmt.Println("Error marshalling observability client args", err)
				return
			}

			client.functions.Increment(data.Metric, data.Value, data.Tags)
		},
		// Gauge
		func(argsPtr *byte, argsLength uint64) {
			data, err := tryMarshalStandardArgs(argsPtr, argsLength)
			if err != nil {
				fmt.Println("Error marshalling observability client args", err)
				return
			}
			client.functions.Gauge(data.Metric, data.Value, data.Tags)
		},
		// Dist
		func(argsPtr *byte, argsLength uint64) {
			data, err := tryMarshalStandardArgs(argsPtr, argsLength)
			if err != nil {
				fmt.Println("Error marshalling observability client args", err)
				return
			}
			client.functions.Dist(data.Metric, data.Value, data.Tags)
		},
		// Error
		func(argsPtr *byte, argsLength uint64) {
			data, err := tryMarshalErrorArgs(argsPtr, argsLength)
			if err != nil {
				fmt.Println("Error marshalling observability client args", err)
				return
			}
			client.functions.Error(data.Tag, data.Error)
		},
		// ShouldEnableHighCardinalityForThisTag
		func(argsPtr *byte, argsLength uint64) bool {
			tag := internal.GoStringFromPointer(argsPtr, argsLength)
			return client.functions.ShouldEnableHighCardinalityForThisTag(*tag)
		},
	)

	runtime.SetFinalizer(client, func(obj *ObservabilityClient) {
		GetFFI().observability_client_release(obj.ref)
	})

	return client
}

func (c *ObservabilityClient) INTERNAL_testObservabilityClient(action string, metricName string, value float64, tags string) {
	GetFFI().__internal__test_observability_client(c.ref, action, metricName, value, tags)
}

func tryMarshalStandardArgs(inputPtr *byte, inputLength uint64) (*obsClientArgs, error) {
	data := internal.GoStringFromPointer(inputPtr, inputLength)

	var args obsClientArgs
	err := json.Unmarshal([]byte(*data), &args)
	if err != nil {
		fmt.Println("Error unmarshalling observability client args", err)
		return nil, err
	}

	return &args, nil
}

func tryMarshalErrorArgs(inputPtr *byte, inputLength uint64) (*obsClientErrorArgs, error) {
	data := internal.GoStringFromPointer(inputPtr, inputLength)

	var args obsClientErrorArgs
	err := json.Unmarshal([]byte(*data), &args)
	if err != nil {
		fmt.Println("Error unmarshalling observability client error args", err)
		return nil, err
	}
	return &args, nil
}
