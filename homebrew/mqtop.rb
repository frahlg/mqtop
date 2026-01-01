# typed: false
# frozen_string_literal: true

class Mqtop < Formula
  desc "High-performance MQTT explorer TUI - like htop for your broker"
  homepage "https://github.com/srcfl/mqtop"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/srcfl/mqtop/releases/download/v#{version}/mqtop-macos-arm64"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"
    end
    on_intel do
      url "https://github.com/srcfl/mqtop/releases/download/v#{version}/mqtop-macos-x64"
      sha256 "PLACEHOLDER_SHA256_MACOS_X64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/srcfl/mqtop/releases/download/v#{version}/mqtop-linux-arm64"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    end
    on_intel do
      url "https://github.com/srcfl/mqtop/releases/download/v#{version}/mqtop-linux-x64"
      sha256 "PLACEHOLDER_SHA256_LINUX_X64"
    end
  end

  def install
    binary_name = "mqtop"

    # The downloaded file might have a platform suffix, rename it
    downloaded_file = Dir["mqtop*"].first || binary_name
    mv downloaded_file, binary_name if downloaded_file != binary_name

    bin.install binary_name
  end

  test do
    assert_match "mqtop", shell_output("#{bin}/mqtop --version 2>&1", 0)
  end
end
