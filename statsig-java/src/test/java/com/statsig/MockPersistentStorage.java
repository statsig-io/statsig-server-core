package com.statsig;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class MockPersistentStorage implements PersistentStorage {
  public List<String> loadCalls = new ArrayList<>();
  public List<SaveCall> saveCalls = new ArrayList<>();
  public List<DeleteCall> deleteCalls = new ArrayList<>();
  private Map<String, Map<String, StickyValues>> storage = new HashMap<>();

  @Override
  public Map<String, StickyValues> load(String key) {
    loadCalls.add(key);
    Map<String, StickyValues> values = storage.get(key);
    if (values == null) {
      return null;
    }
    return new HashMap<>(values);
  }

  @Override
  public void save(String key, String configName, StickyValues data) {
    saveCalls.add(new SaveCall(key, configName, data));
    storage.computeIfAbsent(key, k -> new HashMap<>()).put(configName, data);
  }

  @Override
  public void delete(String key, String configName) {
    deleteCalls.add(new DeleteCall(key, configName));
    Map<String, StickyValues> values = storage.get(key);
    if (values != null) {
      values.remove(configName);
      if (values.isEmpty()) {
        storage.remove(key);
      }
    }
  }

  public static class SaveCall {
    public final String key;
    public final String configName;
    public final StickyValues data;

    public SaveCall(String key, String configName, StickyValues data) {
      this.key = key;
      this.configName = configName;
      this.data = data;
    }
  }

  public static class DeleteCall {
    public final String key;
    public final String configName;

    public DeleteCall(String key, String configName) {
      this.key = key;
      this.configName = configName;
    }
  }
}
