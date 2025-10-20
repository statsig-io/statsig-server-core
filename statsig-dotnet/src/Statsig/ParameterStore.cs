using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public interface IParameterStore
    {
        string Name { get; }
        EvaluationDetails? EvaluationDetails { get; }
        string? GetString(string paramName, string? defaultValue = null);
        long? GetLong(string paramName, long defaultValue);
        double? GetDouble(string paramName, double defaultValue);
        bool? GetBool(string paramName, bool? defaultValue = null);
        Dictionary<string, object> GetDictionary(string paramName, Dictionary<string, object>? defaultValue = null);
        List<object> GetList(string paramName, List<object>? defaultValue = null);
    }

    public class ParameterStore : IParameterStore
    {
        [JsonProperty("name")] public string Name { get; }
        [JsonProperty("details")] public EvaluationDetails? EvaluationDetails { get; }

        private readonly ulong _statsigRef = 0;
        private readonly ulong _userRef = 0;
        private readonly string rawJson;

        private readonly EvaluationOptions? options;

        internal ParameterStore(string rawJson, ulong statsigRef, ulong userRef, EvaluationOptions? options = null)
        {
            _statsigRef = statsigRef;
            _userRef = userRef;
            this.rawJson = rawJson;
            var jsonObject = JObject.Parse(rawJson);
            Name = jsonObject["name"]?.ToString() ?? string.Empty;
            EvaluationDetails = jsonObject["details"]?.ToObject<EvaluationDetails>();
            this.options = options;
        }

        unsafe public string? GetString(string paramName, string? defaultValue = null)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(this.Name);
            var paramNameBytes = Encoding.UTF8.GetBytes(paramName);
            var fallbackBytes = defaultValue != null ? Encoding.UTF8.GetBytes(defaultValue) : null;
            var optionsJson = this.options != null ? JsonConvert.SerializeObject(this.options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* storeNamePtr = storeNameBytes)
            fixed (byte* paramNamePtr = paramNameBytes)
            fixed (byte* fallbackPtr = fallbackBytes)
            fixed (byte* optionsPtr = optionsBytes)
            {
                var res = StatsigFFI.statsig_get_string_parameter_from_parameter_store(_statsigRef, _userRef, storeNamePtr, paramNamePtr, fallbackPtr, optionsPtr);
                return StatsigUtils.ReadStringFromPointer(res) ?? defaultValue;
            }
        }

        unsafe public long? GetLong(string paramName, long defaultValue)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(this.Name);
            var paramNameBytes = Encoding.UTF8.GetBytes(paramName);
            var optionsJson = this.options != null ? JsonConvert.SerializeObject(this.options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* storeNamePtr = storeNameBytes)
            fixed (byte* paramNamePtr = paramNameBytes)
            fixed (byte* optionsPtr = optionsBytes)
            {
                return StatsigFFI.statsig_get_int_parameter_from_parameter_store(_statsigRef, _userRef, storeNamePtr, paramNamePtr, defaultValue, optionsPtr);
            }
        }

        unsafe public double? GetDouble(string paramName, double defaultValue)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(this.Name);
            var paramNameBytes = Encoding.UTF8.GetBytes(paramName);
            var optionsJson = this.options != null ? JsonConvert.SerializeObject(this.options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* storeNamePtr = storeNameBytes)
            fixed (byte* paramNamePtr = paramNameBytes)
            fixed (byte* optionsPtr = optionsBytes)
            {
                return StatsigFFI.statsig_get_float64_parameter_from_parameter_store(_statsigRef, _userRef, storeNamePtr, paramNamePtr, defaultValue, optionsPtr);
            }
        }

        unsafe public bool? GetBool(string paramName, bool? defaultValue = null)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(this.Name);
            var paramNameBytes = Encoding.UTF8.GetBytes(paramName);
            var optionsJson = this.options != null ? JsonConvert.SerializeObject(this.options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            var defaultValueInt = defaultValue.HasValue ? (defaultValue.Value ? 1 : 0) : -1;
            fixed (byte* storeNamePtr = storeNameBytes)
            fixed (byte* paramNamePtr = paramNameBytes)
            fixed (byte* optionsPtr = optionsBytes)
            {
                return StatsigFFI.statsig_get_bool_parameter_from_parameter_store(_statsigRef, _userRef, storeNamePtr, paramNamePtr, defaultValueInt, optionsPtr);
            }
        }

        unsafe public Dictionary<string, object> GetDictionary(string paramName, Dictionary<string, object>? defaultValue = null)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(this.Name);
            var paramNameBytes = Encoding.UTF8.GetBytes(paramName);
            var optionsJson = this.options != null ? JsonConvert.SerializeObject(this.options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            var defaultValueJson = defaultValue != null ? JsonConvert.SerializeObject(defaultValue) : null;
            var defaultValueBytes = defaultValueJson != null ? Encoding.UTF8.GetBytes(defaultValueJson) : null;
            fixed (byte* storeNamePtr = storeNameBytes)
            fixed (byte* paramNamePtr = paramNameBytes)
            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* defaultValuePtr = defaultValueBytes)
            {
                var res = StatsigFFI.statsig_get_object_parameter_from_parameter_store(_statsigRef, _userRef, storeNamePtr, paramNamePtr, defaultValuePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(res);
                return jsonString != null
                    ? JsonConvert.DeserializeObject<Dictionary<string, object>>(jsonString)
                    : defaultValue ?? new Dictionary<string, object>();
            }
        }

        unsafe public List<object> GetList(string paramName, List<object>? defaultValue = null)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(this.Name);
            var paramNameBytes = Encoding.UTF8.GetBytes(paramName);
            var optionsJson = this.options != null ? JsonConvert.SerializeObject(this.options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            var defaultValueJson = defaultValue != null ? JsonConvert.SerializeObject(defaultValue) : null;
            var defaultValueBytes = defaultValueJson != null ? Encoding.UTF8.GetBytes(defaultValueJson) : null;
            fixed (byte* storeNamePtr = storeNameBytes)
            fixed (byte* paramNamePtr = paramNameBytes)
            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* defaultValuePtr = defaultValueBytes)
            {
                var res = StatsigFFI.statsig_get_array_parameter_from_parameter_store(_statsigRef, _userRef, storeNamePtr, paramNamePtr, defaultValuePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(res);
                return jsonString != null
                    ? JsonConvert.DeserializeObject<List<object>>(jsonString)
                    : defaultValue ?? new List<object>();
            }
        }
    }
}