class Iron < Formula
  desc "GTK4 keyboard-driven web browser for BlueAK"
  homepage "https://github.com/Akinus21/Iron"
  version "{{VERSION}}"
  url "{{URL}}"
  sha256 "{{SHA256}}"

  depends_on "gtk4"
  depends_on "libadwaita"

  resource "cef-runtime" do
    url "{{CEF_RUNTIME_URL}}"
    sha256 "{{CEF_RUNTIME_SHA256}}"
  end

  def install
    bin.install "iron"

    resource("cef-runtime").stage do
      lib.install Dir["*.so"]
      (share/"iron").install Dir["*.pak"], "icudtl.dat"
      (share/"iron"/"locales").install Dir["locales/*"] if Dir.exist?("locales")
      (share/"iron").install "v8_context_snapshot.bin" if File.exist?("v8_context_snapshot.bin")
    end
  end

  test do
    assert_match "iron", shell_output("#{bin}/iron --version 2>&1 || true")
  end
end