package statsig

/*
#include "statsig_ffi.h"
#include <stdlib.h>

typedef void (*init_fn_t)(uint64_t);
typedef void (*increment_fn_t)(uint64_t, char*, double, void*);
typedef void (*gauge_fn_t)(uint64_t, char*, double, void*);
typedef void (*dist_fn_t)(uint64_t, char*, double, void*);
typedef void (*error_fn_t)(uint64_t, char*, char*);
typedef bool (*should_enable_high_cardinality_for_this_tag_fn_t)(uint64_t, char*);


void go_observability_client_init(uint64_t obClientID);
void go_observability_client_increment(uint64_t obClientID, char* metricName, double value, char* tags);
void go_observability_client_gauge(uint64_t obClientID, char* metricName, double value, char* tags);
void go_observability_client_dist(uint64_t obClientID, char* metricName, double value, char* tags);
void go_observability_client_error(uint64_t obClientID, char* tag, char* error);
bool go_observability_client_should_enable_high_cardinality_for_this_tag_fn(uint64_t obClientID, char* tag);
*/
import (
	"C"
)
import (
	"fmt"
	"runtime"
	"sync"

	"github.com/statsig-io/statsig-server-core/statsig-go/src/utils"
)

/**
 * IObservabilityClient is an interface for observability clients that allows users to plug in their
 * own observability integration for metrics collection and monitoring.
 */
type ObservabilityClientInterface interface {
	/**
	 * Init initializes the observability client with necessary configuration.
	 * The context parameter allows for cancellation and timeout control.
	 */
	Init() error

	/**
	 * Increment increments a counter metric.
	 * metricName: The name of the metric to increment.
	 * value: The value by which the counter should be incremented (default is 1).
	 * tags: Optional map of tags for metric dimensions.
	 */
	Increment(metricName string, value float64, tags map[string]string) error

	/**
	 * Gauge sets a gauge metric.
	 * metricName: The name of the metric to set.
	 * value: The value to set the gauge to.
	 * tags: Optional map of tags for metric dimensions.
	 */
	Gauge(metricName string, value float64, tags map[string]string) error

	/**
	 * Distribution records a distribution metric for tracking statistical data.
	 * metricName: The name of the metric to record.
	 * value: The recorded value for the distribution metric.
	 * tags: Optional map of tags that represent dimensions to associate with the metric.
	 */
	Distribution(metricName string, value float64, tags map[string]string) error

	/**
	 * ShouldEnableHighCardinalityForThisTag determines if a high cardinality tag should be logged.
	 * tag: The tag to check for high cardinality enabled.
	 */
	ShouldEnableHighCardinalityForThisTag(tag string) bool

	/**
	 * Error
	 */
	Error(tag string, error string)
}

// global registry to store data store implementations indexed by rust data store id
var (
	observabilityClientRegistry = make(map[uint64]ObservabilityClientInterface)
	registryMutexObClient       sync.RWMutex
)

//export go_observability_client_init
func go_observability_client_init(obClientID C.uint64_t) {
	registryMutexObClient.RLock()
	defer registryMutexObClient.RUnlock()

	if obsClient, exists := observabilityClientRegistry[uint64(obClientID)]; exists {
		err := obsClient.Init()
		if err != nil {
			fmt.Println("Error initializing ob client")
		}
	}
}

//export go_observability_client_increment
func go_observability_client_increment(obClientID C.uint64_t, metricName *C.char, value C.double, tags *C.char) {
	registryMutexObClient.RLock()
	defer registryMutexObClient.RUnlock()

	var tags_map map[string]string

	tags_map, _ = utils.ConvertStringToJSON[map[string]string](C.GoString(tags))

	if obsClient, exists := observabilityClientRegistry[uint64(obClientID)]; exists {
		err := obsClient.Increment(C.GoString(metricName), float64(value), tags_map)
		if err != nil {
			fmt.Println("Error incrementing observability client", err)
		}
	}
}

//export go_observability_client_gauge
func go_observability_client_gauge(obClientID C.uint64_t, metricName *C.char, value C.double, tags *C.char) {
	registryMutexObClient.RLock()
	defer registryMutexObClient.RUnlock()

	var tags_map map[string]string

	tags_map, _ = utils.ConvertStringToJSON[map[string]string](C.GoString(tags))

	if obsClient, exists := observabilityClientRegistry[uint64(obClientID)]; exists {
		err := obsClient.Gauge(C.GoString(metricName), float64(value), tags_map)
		if err != nil {
			fmt.Println("Error gaugeing observability client", err)
		}
	}
}

//export go_observability_client_dist
func go_observability_client_dist(obClientID C.uint64_t, metricName *C.char, value C.double, tags *C.char) {
	registryMutexObClient.RLock()
	defer registryMutexObClient.RUnlock()

	var tags_map map[string]string

	tags_map, _ = utils.ConvertStringToJSON[map[string]string](C.GoString(tags))

	if obsClient, exists := observabilityClientRegistry[uint64(obClientID)]; exists {
		err := obsClient.Distribution(C.GoString(metricName), float64(value), tags_map)
		if err != nil {
			fmt.Println("Error distributing observability client", err)
		}
	}
}

//export go_observability_client_error
func go_observability_client_error(obClientID C.uint64_t, tag *C.char, error *C.char) {
	registryMutexObClient.RLock()
	defer registryMutexObClient.RUnlock()

	if obsClient, exists := observabilityClientRegistry[uint64(obClientID)]; exists {
		obsClient.Error(C.GoString(tag), C.GoString(error))
	}
}

//export go_observability_client_should_enable_high_cardinality_for_this_tag_fn
func go_observability_client_should_enable_high_cardinality_for_this_tag_fn(obClientID C.uint64_t, tag *C.char) C.bool {
	registryMutexObClient.RLock()
	defer registryMutexObClient.RUnlock()

	if obsClient, exists := observabilityClientRegistry[uint64(obClientID)]; exists {
		return C.bool(obsClient.ShouldEnableHighCardinalityForThisTag(C.GoString(tag)))
	}

	return C.bool(false)
}

func NewObservabilityClient(observabilityClient ObservabilityClientInterface) uint64 {
	ob_client_ref := C.observability_client_create(
		(C.init_fn_t)(C.go_observability_client_init),
		(C.increment_fn_t)(C.go_observability_client_increment),
		(C.gauge_fn_t)(C.go_observability_client_gauge),
		(C.dist_fn_t)(C.go_observability_client_dist),
		(C.error_fn_t)(C.go_observability_client_error),
		(C.should_enable_high_cardinality_for_this_tag_fn_t)(C.go_observability_client_should_enable_high_cardinality_for_this_tag_fn),
	)

	C.observability_client_set_ref(ob_client_ref, ob_client_ref)

	registryMutexObClient.Lock()
	observabilityClientRegistry[uint64(ob_client_ref)] = observabilityClient
	registryMutexObClient.Unlock()

	if impl_ptr, ok := observabilityClient.(interface{}); ok {
		runtime.SetFinalizer(impl_ptr, func(obj interface{}) {
			C.observability_client_release(ob_client_ref)
		})
	}
	return uint64(ob_client_ref)
}
