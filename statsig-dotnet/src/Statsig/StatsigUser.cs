using System;
using System.Collections.Generic;
using System.Text;
using System.Text.Json;

namespace Statsig
{
    public class StatsigUser : IDisposable
    {
        private ulong _ref;

        internal ulong Reference => _ref;

        public StatsigUser(StatsigUserBuilder builder)
        {
            var userIdBytes = builder.userID != null ? Encoding.UTF8.GetBytes(builder.userID) : Array.Empty<byte>();
            var emailBytes = builder.email != null ? Encoding.UTF8.GetBytes(builder.email) : Array.Empty<byte>();
            var ipBytes = builder.ip != null ? Encoding.UTF8.GetBytes(builder.ip) : Array.Empty<byte>();
            var userAgentBytes = builder.userAgent != null ? Encoding.UTF8.GetBytes(builder.userAgent) : Array.Empty<byte>();
            var countryBytes = builder.country != null ? Encoding.UTF8.GetBytes(builder.country) : Array.Empty<byte>();
            var localeBytes = builder.locale != null ? Encoding.UTF8.GetBytes(builder.locale) : Array.Empty<byte>();
            var appVersionBytes = builder.appVersion != null ? Encoding.UTF8.GetBytes(builder.appVersion) : Array.Empty<byte>();
            var customIDsJson = builder.customIDs != null ? JsonSerializer.Serialize(builder.customIDs) : null;
            var customPropertiesJson = builder.customProperties != null ? JsonSerializer.Serialize(builder.customProperties) : null;
            var privateAttributesJson = builder.privateAttributes != null ? JsonSerializer.Serialize(builder.privateAttributes) : null;

            unsafe
            {
                fixed (byte* userIdPtr = userIdBytes)
                fixed (byte* emailPtr = emailBytes)
                fixed (byte* ipPtr = ipBytes)
                fixed (byte* userAgentPtr = userAgentBytes)
                fixed (byte* countryPtr = countryBytes)
                fixed (byte* localePtr = localeBytes)
                fixed (byte* appVersionPtr = appVersionBytes)
                fixed (byte* customIDsPtr = customIDsJson != null ? Encoding.UTF8.GetBytes(customIDsJson) : null)
                fixed (byte* customPropertiesPtr = customPropertiesJson != null ? Encoding.UTF8.GetBytes(customPropertiesJson) : null)
                fixed (byte* privateAttributesPtr = privateAttributesJson != null ? Encoding.UTF8.GetBytes(privateAttributesJson) : null)
                {
                    _ref = StatsigFFI.statsig_user_create(userIdPtr, customIDsPtr, emailPtr, ipPtr, userAgentPtr, countryPtr, localePtr, appVersionPtr, customPropertiesPtr, privateAttributesPtr);
                }
            }
        }

        ~StatsigUser()
        {
            Dispose(false);
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            StatsigFFI.statsig_user_release(_ref);
        }
    }

    public class StatsigUserBuilder
    {
        internal Dictionary<string, object>? customProperties;
        internal Dictionary<string, object>? privateAttributes;
        internal Dictionary<string, string>? customIDs;
        internal string? userID;
        internal string? email;
        internal string? ip;
        internal string? userAgent;
        internal string? country;
        internal string? locale;
        internal string? appVersion;

        public StatsigUserBuilder()
        {
        }

        public StatsigUserBuilder SetUserID(string userId)
        {
            this.userID = userId;
            return this;
        }
        public StatsigUserBuilder SetEmail(string email)
        {
            this.email = email;
            return this;
        }

        public StatsigUserBuilder SetIP(string ip)
        {
            this.ip = ip;
            return this;
        }
        public StatsigUserBuilder SetUserAgent(string userAgent)
        {
            this.userAgent = userAgent;
            return this;
        }

        public StatsigUserBuilder SetCountry(string country)
        {
            this.country = country;
            return this;
        }

        public StatsigUserBuilder SetLocale(string locale)
        {
            this.locale = locale;
            return this;
        }

        public StatsigUserBuilder SetAppVersion(string appVersion)
        {
            this.appVersion = appVersion;
            return this;
        }

        public StatsigUserBuilder SetCustomIDs(Dictionary<string, string> customIDs)
        {
            this.customIDs = customIDs;
            return this;
        }

        public StatsigUserBuilder AddCustomID(string key, string value)
        {
            if (customIDs == null)
            {
                customIDs = new Dictionary<string, string>();
            }
            customIDs[key] = value;
            return this;
        }

        public StatsigUserBuilder SetCustomProperties(Dictionary<string, object> customProperties)
        {
            this.customProperties = customProperties;
            return this;
        }

        public StatsigUserBuilder AddCustomProperty(string key, object value)
        {
            if (customProperties == null)
            {
                customProperties = new Dictionary<string, object>();
            }
            customProperties[key] = value;
            return this;
        }

        public StatsigUserBuilder SetPrivateAttributes(Dictionary<string, object> privateAttributes)
        {
            this.privateAttributes = privateAttributes;
            return this;
        }

        public StatsigUserBuilder AddPrivateAttribute(string key, object value)
        {
            if (privateAttributes == null)
            {
                privateAttributes = new Dictionary<string, object>();
            }
            privateAttributes[key] = value;
            return this;
        }

        public StatsigUser Build()
        {
            return new StatsigUser(this);
        }
    }
}