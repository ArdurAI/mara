class Mara < Formula
  desc "AI-native telemetry shipper for AI agents and LLM workloads"
  homepage "https://github.com/ArdurAI/mara"
  license "Apache-2.0"
  version "0.1.0"

  on_macos do
    on_arm do
      url "https://github.com/ArdurAI/mara/releases/download/v#{version}/mara-#{version}-aarch64-apple-darwin.tar.gz"
      # sha256 set at release time by the release workflow.
      sha256 "REPLACE_AT_RELEASE_TIME"
    end
    on_intel do
      url "https://github.com/ArdurAI/mara/releases/download/v#{version}/mara-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_AT_RELEASE_TIME"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/ArdurAI/mara/releases/download/v#{version}/mara-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_AT_RELEASE_TIME"
    end
    on_arm do
      url "https://github.com/ArdurAI/mara/releases/download/v#{version}/mara-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_AT_RELEASE_TIME"
    end
  end

  def install
    bin.install "mara"
    (prefix/"share/mara/examples").install_metafiles
  end

  service do
    run [opt_bin/"mara", "run", "--config", "#{etc}/mara/mara.toml"]
    keep_alive true
    log_path var/"log/mara.out.log"
    error_log_path var/"log/mara.err.log"
    working_dir var/"mara"
  end

  test do
    assert_match "mara", shell_output("#{bin}/mara version")
  end
end
