require "dependabot/omnibus"
require 'dependabot/metadata_finders'

credentials =
  [{
    "type" => "git_source",
    "host" => "github.com",
    "username" => "x-access-token",
    "password" => ENV["GITHUB_TOKEN"]
  }]

dependency = Dependabot::Dependency.new(
  name: ENV["DEPENDABOT_PACKAGE"],
  package_manager: ENV["DEPENDABOT_PACKAGE_MANAGER"],
  previous_version: ENV["DEPENDABOT_VERSION"],
  version: ENV["DEPENDABOT_NEW_VERSION"],
  requirements: [],
  previous_requirements: [],
)

metadata_finder = Dependabot::MetadataFinders.for_package_manager("cargo").new(
  dependency: dependency,
  credentials: credentials
)

update_metadata = {
  "changelog_url" => metadata_finder.changelog_url,
  "changelog_text" => metadata_finder.changelog_text,
  "commits_url" => metadata_finder.commits_url,
  "commits" => metadata_finder.commits,
}

puts update_metadata.to_json
