# frozen_string_literal: true

require_relative "lib/model/version"

Gem::Specification.new do |spec|
  spec.name = "smbcloud-model"
  spec.version = Model::VERSION
  spec.authors = ["paydii"]
  spec.email = ["hej@setoelkahfi.se"]

  spec.summary = "Ruby binding for smbcloud-cli model."
  spec.description = "Ruby binding for smbcloud-cli modellllllllllll."
  spec.homepage = "https://github.com/smbcloudXYZ/smbcloud-cli/gems/model"
  spec.required_ruby_version = ">= 3.1.0"
  spec.required_rubygems_version = ">= 3.3.11"

  spec.metadata["allowed_push_host"] = "https://rubygems.org"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/smbcloudXYZ/smbcloud-cli"
  spec.metadata["changelog_uri"] = "https://github.com/smbcloudXYZ/smbcloud-cli/CHANGELOG.md"

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
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

  # Uncomment to register a new dependency of your gem
  # spec.add_dependency "example-gem", "~> 1.0"
  spec.add_dependency "rb_sys", "~> 0.9.91"

  # For more information and examples about making a new gem, check out our
  # guide at: https://bundler.io/guides/creating_gem.html
end
