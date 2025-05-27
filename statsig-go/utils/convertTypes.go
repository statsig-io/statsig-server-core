package utils

import (
	"encoding/json"
	"fmt"
)

func ConvertDataToJson(data interface{}) string {
	jsonValue, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		fmt.Println("Data could not be converted to json", err)
		return ""
	}
	return string(jsonValue)
}
