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
    resource("libcef").stage do
      libexec.install "libcef.so"
    end
  end

  def caveats
    <<~EOS
      Iron requires libcef.so to run. It has been installed to:
        #{libexec}/libcef.so

      You may need to add it to your LD_LIBRARY_PATH:
        export LD_LIBRARY_PATH="#{libexec}:$LD_LIBRARY_PATH"
    EOS
  end

  test do
    assert_match "iron", shell_output("#{bin}/iron --version 2>&1 || true")
  end
end