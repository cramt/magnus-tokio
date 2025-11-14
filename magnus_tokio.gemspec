lib = File.expand_path("lib", __dir__)
$LOAD_PATH.unshift(lib) unless $LOAD_PATH.include?(lib)

Gem::Specification.new do |gem|
  gem.name          = "prometheus_remote_write"
  gem.version       = "0.1.0"
  gem.authors       = ["RE-ZIP"]
  gem.email         = ["developers@re-zip.com"]
  gem.description   = "Ruby gem to do remote writes to prometheus"
  gem.summary       = "Ruby gem to do remote writes to prometheus"
  gem.homepage      = "https://github.com/re-zip/prometheus_remote_write"

  gem.required_ruby_version = ">= 3.0.0"

  gem.files = Dir["lib/**/*.rb", "ext/**/*.{rs,toml,lock,rb}"]
  gem.bindir = "exe"
  gem.executables = gem.files.grep(%r{\Aexe/}) { |f| File.basename(f) }
  gem.require_paths = ["lib"]
  gem.extensions = ["ext/magnus_tokio/extconf.rb"]

  gem.add_dependency "async"
  gem.add_dependency "io-stream"
  gem.add_dependency "rb_sys", "~> 0.9.39"

  gem.add_development_dependency "rake-compiler", "~> 1.2.0"
end
