# frozen_string_literal: true

require_relative 'lib/model/version'

Gem::Specification.new do |spec|
  spec.name = 'smbcloud-model'
  spec.version = Model::VERSION
  spec.authors = ["Seto Elkahfi"]
  spec.email = ["hej@setoelkahfi.se"]

  spec.summary = "Ruby bindings for the smbCloud model types."
  spec.description = "Ruby bindings for shared smbCloud model types, backed by a native Rust extension."
  spec.homepage = "https://github.com/smbcloudXYZ/smbcloud-cli/tree/main/sdk/gems/model"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1.0"
  spec.required_rubygems_version = ">= 3.3.11"

  spec.metadata["allowed_push_host"] = "https://rubygems.org"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/smbcloudXYZ/smbcloud-cli/tree/main/sdk/gems/model"
  spec.metadata["changelog_uri"] = "https://github.com/smbcloudXYZ/smbcloud-cli/tree/main/sdk/gems/model/CHANGELOG.md"

  # Build the release package from tracked files in git.
  gemspec = File.basename(__FILE__)
  spec.files = IO.popen(%w[git ls-files -z], chdir: __dir__, err: IO::NULL) do |ls|
    ls.readlines("\x0", chomp: true).reject do |f|
      (f == gemspec) ||
        f.start_with?(*%w[bin/ test/ spec/ features/ .git appveyor Gemfile])
    end
  end
  spec.bindir = "exe"
  spec.executables = spec.files.grep(%r{\Aexe/}) { |f| File.basename(f) }
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/model/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9.91"
end
