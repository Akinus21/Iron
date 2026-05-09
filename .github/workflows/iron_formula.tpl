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
    bin.install "iron" => "iron.bin"

    resource("cef-runtime").stage do
      files = Dir["*"] + Dir["*/*"].select { |f| File.file?(f) }
      lib.install "libcef.so" if files.any? { |f| File.basename(f) == "libcef.so" }

      Dir.glob("*.dat").each { |f| lib.install f }
      Dir.glob("*.bin").each { |f| lib.install f }
      Dir.glob("*.pak").each { |f| (share/"iron").install f }
      Dir.glob("locales/*").each { |f| (share/"iron"/"locales").install f } if Dir.exist?("locales")
    end

    (bin/"iron").write <<~SH
      #!/bin/bash
      export LD_LIBRARY_PATH="#{lib}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
      exec "#{bin}/iron.bin" "$@"
    SH
  end

  test do
    assert_match "iron", shell_output("#{bin}/iron --version 2>&1 || true")
  end
end