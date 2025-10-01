using System;
using System.Text;
using System.Text.Json;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json;

namespace Statsig
{
    /// <summary>
    /// Configuration options for the GetClientInitializeResponse method in the Statsig Server SDK.
    /// </summary>
    public class ClientInitResponseOptions
    {
        [JsonProperty("hash_algorithm")]
        public string? HashAlgorithm { get; set; }
        [JsonProperty("client_sdk_key")]
        public string? ClientSDKKey { get; set; }
        [JsonProperty("include_local_overrides")]
        public bool IncludeLocalOverrides { get; set; }
        [JsonProperty("remove_id_type")]
        public bool RemoveIdType { get; set; }

        public ClientInitResponseOptions()
        {
        }
    }
}