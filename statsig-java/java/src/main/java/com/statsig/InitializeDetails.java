package com.statsig;

import com.google.gson.annotations.Expose;

public class InitializeDetails {
    public double duration;
    public boolean init_success;
    public boolean is_config_spec_ready;
    public Boolean is_id_list_ready;
    public String source;
    public FailureDetails failure_details;

    @Expose(serialize = false, deserialize = false)
    String rawJson;

    InitializeDetails(double duration, boolean init_success, boolean is_config_spec_ready, boolean is_id_list_ready, String source, FailureDetails failure_details) {
        this.duration = duration;
        this.init_success = init_success;
        this.is_config_spec_ready = is_config_spec_ready;
        this.is_id_list_ready = is_id_list_ready;
        this.source = source;
        this.failure_details = failure_details;
    }

    public double getDuration() {
        return duration;
    }
    
    public boolean getIsInitSuccess() {
        return init_success;
    }
    
    public boolean getIsConfigSpecReady() {
        return is_config_spec_ready;
    }
    
    public Boolean getIsIdListReady() {
        return is_id_list_ready;
    }
    
    public String getSource() {
        return source;
    }
    
    public FailureDetails getFailureDetails() {
        return failure_details;
    }
    
    public String getRawJson() {
        return rawJson;
    }

    void setRawJson(String rawJson) {
        this.rawJson = rawJson;
    }
}

