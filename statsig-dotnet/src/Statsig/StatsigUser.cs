using System;
using System.Collections.Generic;
using System.Text;
using Newtonsoft.Json;

namespace Statsig
{
    public interface IStatsigUser
    {
        string? UserID { get; }
        string? Email { get; }
        string? IP { get; }
        string? UserAgent { get; }
        string? Country { get; }
        string? Locale { get; }
        string? AppVersion { get; }
        IReadOnlyDictionary<string, string>? CustomIDs { get; }
        IReadOnlyDictionary<string, object>? CustomProperties { get; }
        IReadOnlyDictionary<string, object>? PrivateAttributes { get; }
        void Dispose();
        internal ulong Reference { get; }
    }

    public class StatsigUser : IDisposable, IStatsigUser
    {
        public string? UserID { get; }
        public string? Email { get; }
        public string? IP { get; }
        public string? UserAgent { get; }
        public string? Country { get; }
        public string? Locale { get; }
        public string? AppVersion { get; }
        public IReadOnlyDictionary<string, string>? CustomIDs { get; }
        public IReadOnlyDictionary<string, object>? CustomProperties { get; }
        public IReadOnlyDictionary<string, object>? PrivateAttributes { get; }

        private readonly ulong _ref;

        internal ulong Reference => _ref;
        ulong IStatsigUser.Reference => _ref;

        public StatsigUser(StatsigUserBuilder builder)
        {
            // Assign public properties from builder so they are accessible to callers
            UserID = builder.userID;
            Email = builder.email;
            IP = builder.ip;
            UserAgent = builder.userAgent;
            Country = builder.country;
            Locale = builder.locale;
            AppVersion = builder.appVersion;
            CustomIDs = builder.customIDs;
            CustomProperties = builder.customProperties;
            PrivateAttributes = builder.privateAttributes;

            var userIdBytes = builder.userID != null ? Encoding.UTF8.GetBytes(builder.userID) : Array.Empty<byte>();
            var emailBytes = builder.email != null ? Encoding.UTF8.GetBytes(builder.email) : Array.Empty<byte>();
            var ipBytes = builder.ip != null ? Encoding.UTF8.GetBytes(builder.ip) : Array.Empty<byte>();
            var userAgentBytes = builder.userAgent != null ? Encoding.UTF8.GetBytes(builder.userAgent) : Array.Empty<byte>();
            var countryBytes = builder.country != null ? Encoding.UTF8.GetBytes(builder.country) : Array.Empty<byte>();
            var localeBytes = builder.locale != null ? Encoding.UTF8.GetBytes(builder.locale) : Array.Empty<byte>();
            var appVersionBytes = builder.appVersion != null ? Encoding.UTF8.GetBytes(builder.appVersion) : Array.Empty<byte>();
            var customIDsJson = builder.customIDs != null ? JsonConvert.SerializeObject(builder.customIDs) : null;
            var customPropertiesJson = builder.customProperties != null ? JsonConvert.SerializeObject(builder.customProperties) : null;
            var privateAttributesJson = builder.privateAttributes != null ? JsonConvert.SerializeObject(builder.privateAttributes) : null;

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
            customIDs ??= new Dictionary<string, string>();
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
            customProperties ??= new Dictionary<string, object>();
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
            privateAttributes ??= new Dictionary<string, object>();
            privateAttributes[key] = value;
            return this;
        }

        public StatsigUser Build()
        {
            return new StatsigUser(this);
        }
    }
}