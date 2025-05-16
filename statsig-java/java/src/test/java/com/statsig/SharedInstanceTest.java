package com.statsig;

import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.util.concurrent.ExecutionException;
import org.junit.Test;

public class SharedInstanceTest {
  @Test
  public void testCreateSharedInstance() throws InterruptedException, ExecutionException {
    Statsig.newShared("secret-key");
    Statsig s = Statsig.shared();
    assert (s != null);
    s.initialize().get();
  }

  @Test
  public void testDoubleCreation() {
    ByteArrayOutputStream errContent = new ByteArrayOutputStream();
    System.setErr(new PrintStream(errContent));
    Statsig s1 = Statsig.newShared("secret-key");
    Statsig s2 = Statsig.newShared("secret-key1", new StatsigOptions.Builder().build());
    String output = errContent.toString();
    assertTrue(output.contains("[Statsig] Shared instance has been created"));
    assert (s2 != null);
    Statsig s = Statsig.shared();
    assert (s1 == s);
  }

  @Test
  public void testRemove() {
    Statsig s1 = Statsig.newShared("secret-key");
    assert (s1 != null);
    Statsig.removeSharedInstance();
    ByteArrayOutputStream errContent = new ByteArrayOutputStream();
    System.setErr(new PrintStream(errContent));
    Statsig s2 = Statsig.shared();
    assertTrue(s2 != null);
    String output = errContent.toString();
    assertTrue(output.contains("[Statsig] No shared instance has been created yet"));
  }
}
