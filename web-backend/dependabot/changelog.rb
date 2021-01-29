require "dependabot/omnibus"
require 'dependabot/metadata_finders'

credentials =
  [{
    "type" => "git_source",
    "host" => "github.com",
    "username" => "x-access-token",
    "password" => "6f8c56b758fdb7df88d2ce3acd30c1085f4e7b36"
  }]

dependency = Dependabot::Dependency.new(
  name: ENV["DEPENDABOT_PACKAGE"],
  package_manager: "cargo",
  previous_version: ENV["DEPENDABOT_VERSION"],
  version: ENV["DEPENDABOT_NEW_VERSION"],
  requirements: [],
  previous_requirements: [],
)

metadata_finder = Dependabot::MetadataFinders.for_package_manager("cargo").new(
  dependency: dependency,
  credentials: credentials
)

metadata = {
  "changelog_url" => metadata_finder.changelog_url,
  "changelog_text" => metadata_finder.changelog_text,
  "commits_url" => metadata_finder.commits_url,
  "commits_text" => metadata_finder.commits_text,
}

puts metadata.to_json