class Iron < Formula
  desc "GTK4 keyboard-driven web browser for BlueAK"
  homepage "https://github.com/Akinus21/Iron"
  version "{{VERSION}}"
  url "{{URL}}"
  sha256 "{{SHA256}}"

  depends_on "gtk4"
  depends_on "libadwaita"

  resource "libcef" do
    url "{{LIBCEF_URL}}"
    sha256 "{{LIBCEF_SHA256}}"
  end

  def install
    bin.install "iron"
    resource("libcef").stage { bin.install "libcef.so" }
  end

  test do
    assert_match "iron", shell_output("#{bin}/iron --version 2>&1 || true")
  end
end