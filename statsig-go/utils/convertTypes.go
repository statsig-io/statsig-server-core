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

func ConvertStringToJSON(jsonStr string) (map[string]interface{}, error) {
	var result map[string]interface{}
	err := json.Unmarshal([]byte(jsonStr), &result)
	return result, err
}
