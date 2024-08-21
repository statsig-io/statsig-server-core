package com.statsig;

import com.sun.jna.NativeLong;
import com.sun.jna.Structure;

import java.util.Arrays;
import java.util.List;
import java.util.Objects;

public class Ref {
    public final ByReference byReference;
    public final ByValue byValue;
    private static final NativeLong ZERO = new NativeLong(0);

    public Ref(ByReference byReference, ByValue byValue) {
        this.byReference = byReference;
        this.byValue = byValue;
    }

    public boolean isReleased() {
        return Objects.equals(byReference.pointer, ZERO) || Objects.equals(byValue.pointer, ZERO);
    }

    public static class RefData extends Structure {
        public NativeLong pointer;

        @Override
        protected List<String> getFieldOrder() {
            return Arrays.asList("pointer");
        }
    }

    public static class ByReference extends RefData implements Structure.ByReference {
        public Ref toRef() {
            Ref.ByValue byValue = new Ref.ByValue();
            byValue.pointer = this.pointer;
            return new Ref(this, byValue);
        }
    }

    public static class ByValue extends RefData implements Structure.ByValue {
        public Ref toRef() {
            Ref.ByReference byReference = new Ref.ByReference();
            byReference.pointer = this.pointer;
            return new Ref(byReference, this);
        }
    }
}


