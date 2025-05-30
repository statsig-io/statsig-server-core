package com.statsig;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;

public class TestUtils {

  public static String loadJsonFromFile(String fileName) throws IOException {
    try (InputStream inputStream = TestUtils.class.getClassLoader().getResourceAsStream(fileName)) {
      if (inputStream == null) {
        // look for files outside of the /resources directory
        File file = new File(fileName);
        if (file.exists()) {
          try (FileInputStream fileInputStream = new FileInputStream(file)) {
            return readStreamToString(fileInputStream);
          }
        }
        throw new IOException("Resource not found: " + fileName);
      }
      return readStreamToString(inputStream);
    }
  }

  private static String readStreamToString(InputStream inputStream) throws IOException {
    ByteArrayOutputStream result = new ByteArrayOutputStream();
    byte[] buffer = new byte[1024];
    int length;
    while ((length = inputStream.read(buffer)) != -1) {
      result.write(buffer, 0, length);
    }
    return result.toString("UTF-8");
  }
}
