package utils

import (
	"encoding/json"
	"fmt"
)

func ConvertJSONToString(data interface{}) string {
	jsonValue, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		fmt.Println("Data could not be converted to json", err)
		return ""
	}
	return string(jsonValue)
}

func ConvertStringToJSON[T any](jsonStr string) (T, error) {
	var result T
	err := json.Unmarshal([]byte(jsonStr), &result)
	return result, err
}

func ConvertToSafeOptBool(val *bool) int {
	if val == nil {
		return -1
	} else if *val {
		return 1
	} else {
		return 0
	}

}
