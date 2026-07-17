package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import java.util.ArrayList;
import java.util.List;

public class ExperimentGroupsResult {
  /**
   * Null when the name does not refer to an experiment (unknown name or a non-experiment entity
   * like a dynamic config or autotune); otherwise the experiment's isActive state.
   */
  public final Boolean isExperimentActive;

  public final List<ExperimentGroup> groups;

  @JSONCreator
  ExperimentGroupsResult(
      @JSONField(name = "is_experiment_active") Boolean isExperimentActive,
      @JSONField(name = "groups") List<ExperimentGroup> groups) {
    this.isExperimentActive = isExperimentActive;
    this.groups = groups != null ? groups : new ArrayList<>();
  }

  public Boolean getIsExperimentActive() {
    return isExperimentActive;
  }

  public List<ExperimentGroup> getGroups() {
    return groups;
  }
}
