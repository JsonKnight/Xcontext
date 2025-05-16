# frozen_string_literal: true

require 'date'
require 'fileutils'

PROJECT_ROOT = ENV.fetch('PROJECT_ROOT', Dir.pwd)
PROJECT_NAME_STR = ENV.fetch('PROJECT_NAME', File.basename(PROJECT_ROOT))
PROJECT_NAME_SYM = PROJECT_NAME_STR.to_sym
PROJECT_CLI_CRATE_NAME = "#{PROJECT_NAME_STR}-cli"

NEXUS_DIR = ENV.fetch('JSWM_NEXUS_DIR')
INSTALL_DIR = ENV.fetch('PROJECT_INSTALL_DIR', "#{NEXUS_DIR}/xtools")
INSTALL_PATH = File.join(INSTALL_DIR, PROJECT_NAME_STR)

CARGO_TARGET_DIR = ENV.fetch('CARGO_TARGET_DIR', File.join(PROJECT_ROOT, 'target'))
RELEASE_BINARY_PATH = File.join(CARGO_TARGET_DIR, 'release', PROJECT_NAME_STR)

FISH_COMPLETION_DIR = File.expand_path('~/.config/fish/completions')
FISH_COMPLETION_FILE = File.join(FISH_COMPLETION_DIR, "#{PROJECT_NAME_STR}.fish")

Dir.glob(File.join(PROJECT_ROOT, 'tasks/**/*.rake')).sort.each { |r| import r }

namespace PROJECT_NAME_SYM do
  desc 'Check for errors'
  task :check do
    Dir.chdir(PROJECT_ROOT) { sh 'cargo check --all --all-features' }
  end

  desc 'Build all crates (debug)'
  task :build do
    Dir.chdir(PROJECT_ROOT) { sh 'cargo build --all --all-features' }
  end

  desc 'Build release version'
  task :build_release do
    puts "Building release version of #{PROJECT_NAME_STR}..."
    Dir.chdir(PROJECT_ROOT) do
      sh "cargo build --release --all-features --target-dir #{CARGO_TARGET_DIR}"
    end
    raise "Release build failed: Main binary #{RELEASE_BINARY_PATH} not found." unless File.exist?(RELEASE_BINARY_PATH)

    puts "Release build successful: #{RELEASE_BINARY_PATH}"
  end

  desc 'Run tests'
  task :test do
    Dir.chdir(PROJECT_ROOT) { sh 'cargo test --all --all-features' }
  end

  desc 'Run CLI with default project config to sync all projects'
  task :run do
    Dir.chdir(PROJECT_ROOT) { sh "cargo run -p #{PROJECT_CLI_CRATE_NAME} --all-features -- sync" }
  end

  desc "Run CLI to sync a specific project (e.g., rake #{PROJECT_NAME_SYM}:run_project[MyProject1])"
  task :run_project, [:project_name] do |_t, args|
    project_name_arg = args[:project_name]
    raise "Project name is required. Usage: rake #{PROJECT_NAME_SYM}:run_project[ProjectName]" unless project_name_arg

    Dir.chdir(PROJECT_ROOT) do
      sh "cargo run -p #{PROJECT_CLI_CRATE_NAME} --all-features -- sync --project #{project_name_arg}"
    end
  end

  desc 'Format code'
  task :fmt do
    Dir.chdir(PROJECT_ROOT) { sh 'cargo fmt --all' }
  end

  desc 'Lint with clippy'
  task :lint do
    Dir.chdir(PROJECT_ROOT) do
      sh 'cargo clippy --all --all-features -- -D warnings -A clippy::style -A clippy::complexity -A clippy::perf -A clippy::pedantic -A clippy::restriction -A clippy::nursery -A clippy::cargo'
    end
  end

  desc 'Generate documentation'
  task :doc do
    Dir.chdir(PROJECT_ROOT) { sh 'cargo doc --open --all-features --no-deps' }
  end

  desc 'Generate Fish shell completion'
  task :gen_fish_completion do
    Rake::Task["#{PROJECT_NAME_SYM}:build_release"].invoke unless File.exist?(RELEASE_BINARY_PATH)
    unless File.exist?(RELEASE_BINARY_PATH)
      raise "Cannot generate completions: Release binary #{RELEASE_BINARY_PATH} not found."
    end

    puts "Generating Fish completion script using #{RELEASE_BINARY_PATH}..."
    completion_script = `"#{RELEASE_BINARY_PATH}" completion`
    unless $?.success? && !completion_script.empty?
      raise "Failed to generate fish completion script using #{RELEASE_BINARY_PATH}. Output: #{completion_script}"
    end

    FileUtils.mkdir_p(FISH_COMPLETION_DIR) unless Dir.exist?(FISH_COMPLETION_DIR)
    File.write(FISH_COMPLETION_FILE, completion_script)
    puts "Fish completion script written to #{FISH_COMPLETION_FILE}"
  end

  desc 'Convert Mermaid .mmd files to versioned SVG'
  task :mermaid do
    Dir.chdir(PROJECT_ROOT) do
      timestamp = Date.today.strftime('%Y%m%d')
      Dir.glob('.mermaid/mmd/*.mmd').each do |mmd|
        base_name = File.basename(mmd, '.mmd')
        svg_dir = File.join(PROJECT_ROOT, '.mermaid', 'diagrams')
        FileUtils.mkdir_p(svg_dir) unless Dir.exist?(svg_dir)
        svg = File.join(svg_dir, "#{base_name}_#{timestamp}.svg")
        sh "mmdc -i \"#{mmd}\" -o \"#{svg}\""
      end
    end
  end

  desc 'Tangle Org-mode files'
  task :org do
    Dir.chdir(PROJECT_ROOT) do
      Dir.glob(File.join(PROJECT_ROOT, '.org', '*.org')).each do |org_file|
        sh "emacs --batch --no-init-file --no-site-file --eval \"(require 'org)\" --eval \"(org-babel-tangle-file \\\"#{org_file}\\\")\""
      end
      mermaid_org_file = File.join(PROJECT_ROOT, '.mermaid', 'MERMAID.org')
      if File.exist?(mermaid_org_file)
        sh "emacs --batch --no-init-file --no-site-file --eval \"(require 'org)\" --eval \"(org-babel-tangle-file \\\"#{mermaid_org_file}\\\")\""
      end
    end
  end

  desc 'Format, Lint, Check, Build Release, Install CLI, Generate Completion'
  task deploy: %i[fmt lint check build_release] do
    puts "Deploying #{PROJECT_NAME_STR}..."

    FileUtils.mkdir_p(INSTALL_DIR) unless Dir.exist?(INSTALL_DIR)
    puts "Copying CLI binary #{RELEASE_BINARY_PATH} to #{INSTALL_PATH}..."
    FileUtils.cp(RELEASE_BINARY_PATH, INSTALL_PATH, verbose: true)
    FileUtils.chmod(0o755, INSTALL_PATH)

    Rake::Task["#{PROJECT_NAME_SYM}:gen_fish_completion"].invoke

    puts "#{PROJECT_NAME_STR} deployed successfully!"
    puts "- CLI installed to: #{INSTALL_PATH}"
    puts "- Fish completions installed to: #{FISH_COMPLETION_FILE}"
    puts "Ensure #{INSTALL_DIR} is in your $PATH."
  end

  default_action_tasks = %i[check build run]
  desc "Default task: #{default_action_tasks.join(', ')}"
  task default_action: default_action_tasks
end

desc 'Setup project (install Rust, update, build dependencies, create default config)'
task :setup do
  sh 'rustup update stable' if system('which rustup > /dev/null 2>&1')
  Dir.chdir(PROJECT_ROOT) { sh 'cargo build --all --all-features' }
  puts "Project setup complete. Dependencies built. Default config is at #{DEFAULT_CONFIG_FILE}"
rescue StandardError => e
  puts "Setup failed: #{e.message}"
end

desc "Default task: Runs #{PROJECT_NAME_SYM}:default_action"
task default: ["#{PROJECT_NAME_SYM}:default_action"]

task r: ["#{PROJECT_NAME_SYM}:run"]
task c: ["#{PROJECT_NAME_SYM}:check"]
task t: ["#{PROJECT_NAME_SYM}:test"]
task b: ["#{PROJECT_NAME_SYM}:build"]
task br: ["#{PROJECT_NAME_SYM}:build_release"]
task f: ["#{PROJECT_NAME_SYM}:fmt"]
task l: ["#{PROJECT_NAME_SYM}:lint"]
task d: ["#{PROJECT_NAME_SYM}:deploy"]
task fish: ["#{PROJECT_NAME_SYM}:gen_fish_completion"]
task syncall: ["#{PROJECT_NAME_SYM}:run"]
task m: ["#{PROJECT_NAME_SYM}:mermaid"]
task o: ["#{PROJECT_NAME_SYM}:org"]
