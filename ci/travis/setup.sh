#!/bin/sh

set -e +x

# Find the installed version of a binary, if any
_installed() {
    VERSION=$($@ --version 2>/dev/null || echo "$@ none")
    echo $VERSION | rev | cut -d' ' -f1 | rev
}

# Find the latest available version of a binary on `crates.io`
_latest() {
    VERSION=$(cargo search -q "$@" | grep "$@" | cut -f2 -d"\"")
    echo $VERSION
}


### Setup sccache ##############################################################

echo -n "Fetching latest available 'sccache' version... "
INSTALLED=$(_installed sccache)
LATEST=$(_latest sccache)
echo "${LATEST} (installed: ${INSTALLED})"

if [ "$INSTALLED" = "$LATEST" ]; then
  echo "Using cached 'sccache'"
else
  echo "Installing latest 'sccache' from mozilla/sccache"
  URL="https://github.com/mozilla/sccache/releases/download/${LATEST}/sccache-${LATEST}-x86_64-unknown-linux-musl.tar.gz"
  curl -SsL $URL | tar xzv -C /tmp
  mv /tmp/sccache-${LATEST}-x86_64-unknown-linux-musl/sccache $HOME/.cargo/bin/sccache
fi

mkdir -p $SCCACHE_DIR


### Setup cargo-make ###########################################################

echo -n "Fetching latest available 'cargo-make' version..."
INSTALLED=$(_installed cargo make)
LATEST=$(_latest cargo-make)
echo "${LATEST} (installed: ${INSTALLED})"

if [ "$INSTALLED" = "$LATEST" ]; then
  echo "Using cached 'cargo-make'"
else
  echo "Installing latest 'cargo-make' from source"
  cargo install --debug -f cargo-make
fi

mkdir -p $SCCACHE_DIR


### Setup vdpm #################################################################

echo "Fetching latest Vita SDK..."

cd "$VDPM_GIT_DIR"

if [ -d ".git" ]; then
  git pull
else
  git clone https://github.com/vitasdk/vdpm .
fi

. include/install-vitasdk.sh
. include/install-packages.sh

echo "Installing dependencies..."
sudo apt-get install libc6-i386 lib32stdc++6 lib32gcc1 patch

echo "Downloading toolchain..."
mkdir -p $VITASDK
curl -SsL "$(get_download_link linux)" | tar xj -C $VITASDK --strip-components=1

./install-all.sh
