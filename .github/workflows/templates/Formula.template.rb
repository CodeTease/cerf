class {{class_name}} < Formula
  desc "{{description}}"
  homepage "{{homepage}}"
  version "{{version}}"
  license "{{license}}"

  if OS.mac?
    if Hardware::CPU.intel?
      url "{{url_mac_amd}}"
      sha256 "{{sha256_mac_amd}}"
    elsif Hardware::CPU.arm?
      url "{{url_mac_arm}}"
      sha256 "{{sha256_mac_arm}}"
    end
  elsif OS.linux?
    if Hardware::CPU.intel?
      url "{{url_linux_amd}}"
      sha256 "{{sha256_linux_amd}}"
    elsif Hardware::CPU.arm?
      url "{{url_linux_arm}}"
      sha256 "{{sha256_linux_arm}}"
    end
  end

  def install
    bin.install "{{bin}}"
  end
end
