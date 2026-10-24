# frozen_string_literal: true

require_relative 'model/version'
# Fat native gems ship one extension per Ruby minor (lib/model/3.4/model.bundle);
# source and single-ABI builds put it directly under lib/model/.
begin
  RUBY_VERSION =~ /(\d+\.\d+)/
  require_relative "model/#{Regexp.last_match(1)}/model"
rescue LoadError
  require_relative 'model/model'
end

module Model
  class Error < StandardError; end
end
