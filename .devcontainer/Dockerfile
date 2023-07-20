ARG VARIANT="nightly-bookworm-slim"
FROM rustlang/rust:${VARIANT}
ENV DEBIAN_FRONTEND noninteractive
RUN echo 'debconf debconf/frontend select Noninteractive' | debconf-set-selections

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive

RUN apt-get -qq install build-essential libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev