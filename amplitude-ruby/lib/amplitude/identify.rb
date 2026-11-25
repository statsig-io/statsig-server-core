# frozen_string_literal: true

class Amplitude
  class Identify
    def initialize
      @operations = []
    end

    %i[set set_once append prepend pre_insert post_insert remove add].each do |operation|
      define_method(operation) do |key, value|
        @operations << { operation: operation, key: key, value: value }
        self
      end
    end

    def unset(key)
      @operations << { operation: :unset, key: key }
      self
    end

    def clear_all
      @operations << { operation: :clear_all }
      self
    end

    def to_a
      @operations.dup
    end
  end
end
