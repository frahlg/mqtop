# typed: false
# frozen_string_literal: true

class Mqtop < Formula
  desc "High-performance MQTT explorer TUI - like htop for your broker"
  homepage "https://github.com/frahlg/mqtop"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/frahlg/mqtop/releases/download/v#{version}/mqtop-macos-arm64"
      sha256 "a2cc0367c8dbf8a6b2ed4b95c52edfda4afa4428b6de3229cee858d43f080bd3"
    end
    on_intel do
      url "https://github.com/frahlg/mqtop/releases/download/v#{version}/mqtop-macos-x64"
      sha256 "a48774ef095f4f1b6814b5309b7569374e3616f3fce2b515035adf37daec588c"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/frahlg/mqtop/releases/download/v#{version}/mqtop-linux-arm64"
      sha256 "2bb3ead0eca6e410769604242bcc4bca2854fbaefe8539b5f6e7d492d36dab1b"
    end
    on_intel do
      url "https://github.com/frahlg/mqtop/releases/download/v#{version}/mqtop-linux-x64"
      sha256 "be64385b9e04b92ed771f3bdc97ca8d703f224e14ceb7763c4eaefc6e0e30a78"
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
