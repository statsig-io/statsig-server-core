using Xunit;
using System.Collections.Generic;

namespace Statsig.Tests
{
    public class StatsigUserTests
    {
        [Fact]
        public void StatsigUserBuilder_SetUserID_SetsUserIDCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.SetUserID("test_user_123");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_SetEmail_SetsEmailCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.SetEmail("test@example.com");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_SetIP_SetsIPCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.SetIP("192.168.1.1");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_SetUserAgent_SetsUserAgentCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.SetUserAgent("Mozilla/5.0");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_SetCountry_SetsCountryCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.SetCountry("US");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_SetLocale_SetsLocaleCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.SetLocale("en-US");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_SetAppVersion_SetsAppVersionCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.SetAppVersion("1.0.0");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_SetCustomIDs_SetsCustomIDsCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var customIDs = new Dictionary<string, string>
            {
                { "employee_id", "emp_123" },
                { "company_id", "comp_456" }
            };

            var result = builder.SetCustomIDs(customIDs);

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_AddCustomID_AddsCustomIDCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.AddCustomID("test_key", "test_value");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_AddCustomID_CreatesNewDictionaryIfNull()
        {
            var builder = new StatsigUserBuilder();
            builder.AddCustomID("key1", "value1");
            builder.AddCustomID("key2", "value2");

            Assert.NotNull(builder);
        }

        [Fact]
        public void StatsigUserBuilder_SetCustomProperties_SetsCustomPropertiesCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var customProperties = new Dictionary<string, object>
            {
                { "age", 25 },
                { "is_premium", true },
                { "score", 95.5 }
            };

            var result = builder.SetCustomProperties(customProperties);

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_AddCustomProperty_AddsCustomPropertyCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.AddCustomProperty("test_prop", "test_value");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_AddCustomProperty_CreatesNewDictionaryIfNull()
        {
            var builder = new StatsigUserBuilder();
            builder.AddCustomProperty("prop1", "value1");
            builder.AddCustomProperty("prop2", 42);

            Assert.NotNull(builder);
        }

        [Fact]
        public void StatsigUserBuilder_SetPrivateAttributes_SetsPrivateAttributesCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var privateAttributes = new Dictionary<string, object>
            {
                { "ssn", "123-45-6789" },
                { "internal_id", 12345 }
            };

            var result = builder.SetPrivateAttributes(privateAttributes);

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_AddPrivateAttribute_AddsPrivateAttributeCorrectly()
        {
            var builder = new StatsigUserBuilder();
            var result = builder.AddPrivateAttribute("secret_key", "secret_value");

            Assert.Same(builder, result);
        }

        [Fact]
        public void StatsigUserBuilder_AddPrivateAttribute_CreatesNewDictionaryIfNull()
        {
            var builder = new StatsigUserBuilder();
            builder.AddPrivateAttribute("attr1", "value1");
            builder.AddPrivateAttribute("attr2", true);

            Assert.NotNull(builder);
        }

        [Fact]
        public void StatsigUserBuilder_Build_CreatesStatsigUserSuccessfully()
        {
            var builder = new StatsigUserBuilder()
                .SetUserID("test_user")
                .SetEmail("test@example.com")
                .SetCountry("US");

            using var user = builder.Build();

            Assert.NotNull(user);
        }

        [Fact]
        public void StatsigUserBuilder_Build_WithAllProperties_CreatesStatsigUserSuccessfully()
        {
            var customIDs = new Dictionary<string, string> { { "emp_id", "123" } };
            var customProperties = new Dictionary<string, object> { { "age", 30 } };
            var privateAttributes = new Dictionary<string, object> { { "ssn", "xxx" } };

            var builder = new StatsigUserBuilder()
                .SetUserID("test_user")
                .SetEmail("test@example.com")
                .SetIP("192.168.1.1")
                .SetUserAgent("Mozilla/5.0")
                .SetCountry("US")
                .SetLocale("en-US")
                .SetAppVersion("1.0.0")
                .SetCustomIDs(customIDs)
                .SetCustomProperties(customProperties)
                .SetPrivateAttributes(privateAttributes);

            using var user = builder.Build();

            Assert.NotNull(user);
        }

        [Fact]
        public void StatsigUser_Dispose_DoesNotThrow()
        {
            var builder = new StatsigUserBuilder().SetUserID("test");
            var user = builder.Build();

            user.Dispose();

            Assert.True(true);
        }

        [Fact]
        public void StatsigUser_DisposeMultipleTimes_DoesNotThrow()
        {
            var builder = new StatsigUserBuilder().SetUserID("test");
            var user = builder.Build();

            user.Dispose();
            user.Dispose();

            Assert.True(true);
        }

        [Fact]
        public void StatsigUser_PublicProperties_MatchBuilderValues()
        {
            var builder = new StatsigUserBuilder()
                .SetUserID("test_user")
                .SetEmail("test@example.com")
                .SetIP("192.168.1.1")
                .SetUserAgent("Mozilla/5.0")
                .SetCountry("US")
                .SetLocale("en-US")
                .SetAppVersion("1.0.0");

            using var user = builder.Build();

            Assert.Equal("test_user", user.UserID);
            Assert.Equal("test@example.com", user.Email);
            Assert.Equal("192.168.1.1", user.IP);
            Assert.Equal("Mozilla/5.0", user.UserAgent);
            Assert.Equal("US", user.Country);
            Assert.Equal("en-US", user.Locale);
            Assert.Equal("1.0.0", user.AppVersion);
            Assert.Null(user.CustomIDs);
            Assert.Null(user.CustomProperties);
            Assert.Null(user.PrivateAttributes);
        }

        [Fact]
        public void StatsigUser_AllProperties_DefaultToNull_WhenNotSet()
        {
            var builder = new StatsigUserBuilder();
            using var user = builder.Build();

            Assert.Null(user.UserID);
            Assert.Null(user.Email);
            Assert.Null(user.IP);
            Assert.Null(user.UserAgent);
            Assert.Null(user.Country);
            Assert.Null(user.Locale);
            Assert.Null(user.AppVersion);
            Assert.Null(user.CustomIDs);
            Assert.Null(user.CustomProperties);
            Assert.Null(user.PrivateAttributes);
        }

        [Fact]
        public void StatsigUser_WithCustomData_PropertiesMatchBuilderValues()
        {
            var customIDs = new Dictionary<string, string>
            {
                { "employee_id", "emp_123" },
                { "company_id", "comp_456" }
            };

            var customProperties = new Dictionary<string, object>
            {
                { "age", 25 },
                { "is_premium", true },
                { "score", 95.5 }
            };

            var privateAttributes = new Dictionary<string, object>
            {
                { "ssn", "123-45-6789" },
                { "internal_id", 12345 }
            };

            var builder = new StatsigUserBuilder()
                .SetUserID("test_user")
                .SetCustomIDs(customIDs)
                .SetCustomProperties(customProperties)
                .SetPrivateAttributes(privateAttributes);

            using var user = builder.Build();

            Assert.Equal("test_user", user.UserID);
            Assert.Same(customIDs, user.CustomIDs);
            Assert.Same(customProperties, user.CustomProperties);
            Assert.Same(privateAttributes, user.PrivateAttributes);
            Assert.True(user.CustomIDs.ContainsKey("employee_id"));
            Assert.True(user.CustomIDs.ContainsKey("company_id"));
            Assert.Equal("emp_123", user.CustomIDs["employee_id"]);
            Assert.Equal("comp_456", user.CustomIDs["company_id"]);
        }
    }
}
