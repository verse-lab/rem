## Custom Dockerfile
FROM dorowu/ubuntu-desktop-lxde-vnc

RUN useradd -s /sbin/nologin -c "REM tester" tester && mkdir -p /home/tester/rem/

COPY intellij-rust-0.4.188.SNAPSHOT-223-dev.zip /home/tester/rem/
COPY setup.sh /home/tester/rem/

RUN apt-get install -y curl software-properties-common wget
RUN wget -q -O - https://dl.google.com/linux/linux_signing_key.pub | apt-key add -
RUN apt-add-repository ppa:swi-prolog/stable
RUN apt-get update
RUN apt-get install -y swi-prolog

RUN chown -hR tester /home/tester/

USER tester

ENV HOME=/home/tester

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

COPY capstone.conf /etc/ld.so.conf.d/

RUN export PATH=$PATH:/home/tester/.cargo/bin

RUN /home/tester/rem/setup.sh

RUN ldconfig

USER root

