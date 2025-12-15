package com.statsig;

import java.util.Map;

public interface PersistentStorage {
  /**
   * Load persisted values for the given key.
   *
   * @param key The storage key
   * @return Map of config names to StickyValues, or null if no data exists
   */
  Map<String, StickyValues> load(String key);

  /**
   * Save persisted values for the given key and config name.
   *
   * @param key The storage key
   * @param configName The name of the experiment
   * @param data The StickyValues object to save
   */
  void save(String key, String configName, StickyValues data);

  /**
   * Delete persisted values for the given key and config name.
   *
   * @param key The storage key
   * @param configName The name of the experiment
   */
  void delete(String key, String configName);

  /**
   * Helper method to get persisted values for a user and ID type. This method automatically
   * generates the storage key from the user and ID type.
   *
   * @param user The StatsigUser to get persisted values for
   * @param idType The ID type (e.g., "userID", "email", etc.)
   * @return Map of config names to StickyValues, or null if no data exists or key cannot be
   *     generated
   */
  default Map<String, StickyValues> getValuesForUser(StatsigUser user, String idType) {
    String key = getStorageKey(user, idType);
    if (key == null) {
      return null;
    }
    return load(key);
  }

  static String getStorageKey(StatsigUser user, String idType) {
    if (user == null || idType == null) {
      return null;
    }

    String lowerIdType = idType.toLowerCase();
    if (lowerIdType.equals("user_id") || lowerIdType.equals("userid")) {
      String userId = user.getUserID();
      if (userId == null) {
        userId = "";
      }
      return userId + ":userID";
    }

    Map<String, String> customIDs = user.getCustomIDs();
    String customId = "";
    if (customIDs != null) {
      customId = customIDs.get(idType);
      if (customId == null) {
        customId = "";
      }
    }

    return customId + ":" + idType;
  }
}
