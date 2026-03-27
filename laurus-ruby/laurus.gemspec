Gem::Specification.new do |spec|
  spec.name          = "laurus"
  spec.version       = "0.0.0"
  spec.authors       = ["Minoru Osuka"]
  spec.email         = ["minoru.osuka@gmail.com"]
  spec.summary       = "Ruby bindings for the Laurus search library"
  spec.description   = "Unified lexical, vector, and hybrid search for Ruby, powered by Rust."
  spec.homepage      = "https://github.com/mosuka/laurus"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.1"

  spec.files = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rs,toml,rb,lock}",
    "Cargo.*",
    "README.md",
    "LICENSE",
  ]
  spec.extensions    = ["ext/laurus_ruby/extconf.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "rb_sys", "~> 0.9"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "rake-compiler", "~> 1.2"
end
