// <auto-generated>
// This code is generated by csbindgen.
// DON'T CHANGE THIS DIRECTLY.
// </auto-generated>
#pragma warning disable CS8500
#pragma warning disable CS8981
using System;
using System.Runtime.InteropServices;


namespace Statsig
{
    internal static unsafe partial class StatsigFFI
    {
        const string __DllName = "libstatsig_ffi";



        [DllImport(__DllName, EntryPoint = "statsig_options_create", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_options_create(byte* specs_url, byte* log_event_url, byte* specs_adapter_ref, byte* event_logging_adapter_ref);

        [DllImport(__DllName, EntryPoint = "statsig_options_release", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_options_release(byte* options_ref);

        [DllImport(__DllName, EntryPoint = "statsig_user_create", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_user_create(byte* user_id, byte* custom_ids_json, byte* email, byte* ip, byte* user_agent, byte* country, byte* locale, byte* app_version, byte* custom_json, byte* private_attributes_json);

        [DllImport(__DllName, EntryPoint = "statsig_user_release", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_user_release(byte* user_ref);

        [DllImport(__DllName, EntryPoint = "statsig_create", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_create(byte* sdk_key, byte* options_ref);

        [DllImport(__DllName, EntryPoint = "statsig_release", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_release(byte* statsig_ref);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void statsig_initialize_callback_delegate();

        [DllImport(__DllName, EntryPoint = "statsig_initialize", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_initialize(byte* statsig_ref, statsig_initialize_callback_delegate callback);

        [DllImport(__DllName, EntryPoint = "statsig_initialize_blocking", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_initialize_blocking(byte* statsig_ref);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void statsig_flush_events_callback_delegate();

        [DllImport(__DllName, EntryPoint = "statsig_flush_events", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_flush_events(byte* statsig_ref, statsig_flush_events_callback_delegate callback);

        [DllImport(__DllName, EntryPoint = "statsig_flush_events_blocking", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_flush_events_blocking(byte* statsig_ref);

        [DllImport(__DllName, EntryPoint = "statsig_get_current_values", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_get_current_values(byte* statsig_ref);

        [DllImport(__DllName, EntryPoint = "statsig_log_event", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_log_event(byte* statsig_ref, byte* user_ref, byte* event_json);

        [DllImport(__DllName, EntryPoint = "statsig_check_gate", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.U1)]
        internal static extern bool statsig_check_gate(byte* statsig_ref, byte* user_ref, byte* gate_name);

        [DllImport(__DllName, EntryPoint = "statsig_get_feature_gate", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_get_feature_gate(byte* statsig_ref, byte* user_ref, byte* gate_name);

        [DllImport(__DllName, EntryPoint = "statsig_get_dynamic_config", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_get_dynamic_config(byte* statsig_ref, byte* user_ref, byte* config_name);

        [DllImport(__DllName, EntryPoint = "statsig_get_experiment", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_get_experiment(byte* statsig_ref, byte* user_ref, byte* experiment_name);

        [DllImport(__DllName, EntryPoint = "statsig_get_layer", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_get_layer(byte* statsig_ref, byte* user_ref, byte* layer_name);

        [DllImport(__DllName, EntryPoint = "statsig_log_layer_param_exposure", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void statsig_log_layer_param_exposure(byte* statsig_ref, byte* layer_json, byte* param_name);

        [DllImport(__DllName, EntryPoint = "statsig_get_client_init_response", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* statsig_get_client_init_response(byte* statsig_ref, byte* user_ref);


    }



}
