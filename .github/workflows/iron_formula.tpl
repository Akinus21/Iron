class Iron < Formula
  desc "GTK4 keyboard-driven web browser for BlueAK"
  homepage "https://github.com/Akinus21/Iron"
  version "{{VERSION}}"
  url "{{URL}}"
  sha256 "{{SHA256}}"

  depends_on "gtk4"
  depends_on "libadwaita"

  resource "servo-runner" do
    url "{{SERVO_URL}}"
    sha256 "{{SERVO_SHA256}}"
  end

  def install
    bin.install "iron" => "iron.bin"
    libexec.install resource("servo-runner")
    bin.install_symlink libexec/"servo-runner" => "servo-runner"

    (bin/"iron").write <<~SH
      #!/bin/bash
      cd "$(dirname "#{bin}/iron.bin")"
      exec ./iron.bin "$@"
    SH
  end

  test do
    assert_match "iron", shell_output("#{bin}/iron --version 2>&1 || true")
  end
end