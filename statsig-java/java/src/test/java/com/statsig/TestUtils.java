package com.statsig;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;

public class TestUtils {
  public static String loadJsonFromFile(String fileName) throws IOException {
    try (InputStream inputStream = TestUtils.class.getClassLoader().getResourceAsStream(fileName)) {
      if (inputStream == null) {
        // look for files outside of the /resources directory
        File file = new File(fileName);
        if (file.exists()) {
          try (FileInputStream fileInputStream = new FileInputStream(file)) {
            return new String(fileInputStream.readAllBytes(), StandardCharsets.UTF_8);
          }
        }
        throw new IOException("Resource not found: " + fileName);
      }
      return new String(inputStream.readAllBytes(), StandardCharsets.UTF_8);
    }
  }
}
