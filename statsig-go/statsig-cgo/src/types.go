package statsig

type Event struct {
	EventName string            `json:"eventName"`
	User      StatsigUser       `json:"user"`
	Value     interface{}       `json:"value"`
	Metadata  map[string]string `json:"metadata"`
	Time      int64             `json:"time"`
}

type EvaluationDetails struct {
	ReceivedAt int64  `json:"received_at"`
	Lcut       int64  `json:"lcut"`
	Reason     string `json:"reason"`
}
