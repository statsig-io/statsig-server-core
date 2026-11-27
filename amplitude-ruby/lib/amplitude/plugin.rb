# frozen_string_literal: true

class Amplitude
  Result = Struct.new(:event, :code, :message, keyword_init: true)

  class BasePlugin
    attr_reader :plugin_type

    def initialize(plugin_type)
      @plugin_type = plugin_type.to_sym
    end

    def setup(_client); end

    def execute(_event)
      raise NotImplementedError, 'Plugins must implement execute'
    end
  end

  class BeforePlugin < BasePlugin
    def initialize
      super(:before)
    end
  end

  class EnrichmentPlugin < BasePlugin
    def initialize
      super(:enrichment)
    end
  end

  class DestinationPlugin < BasePlugin
    def initialize
      super(:destination)
    end

    def flush; end

    def shutdown
      flush
      nil
    end
  end
end
