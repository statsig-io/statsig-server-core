package main

import (
	"fmt"
	"time"

	statsig "github.com/statsig-io/private-statsig-server-core/statsig-go/src"
)

func main() {
	for i := 0; i < 100; i++ {
		user := statsig.NewStatsigUserBuilder().
			WithUserID("test-user0").
			WithEmail("test@test.com").
			WithIpAddress("127.0.0.1").
			WithUserAgent("test-user-agent").
			WithCountry("US").
			WithLocale("en-US").
			WithAppVersion("1.0.0").
			WithCustom(map[string]interface{}{}).
			WithPrivateAttributes(map[string]interface{}{}).
			WithStatsigEnvironment(map[string]string{}).
			WithCustomIds(map[string]string{}).
			Build()
		fmt.Println(user.GetInnerRef())
		user = nil
		time.Sleep(1 * time.Second)
	}
}
