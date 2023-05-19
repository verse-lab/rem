## Custom Dockerfile
FROM dorowu/ubuntu-desktop-lxde-vnc

RUN useradd -s /sbin/nologin -c "REM tester" tester && mkdir -p /home/tester
RUN usermod -a -G sudo tester
RUN apt-get install -y curl software-properties-common wget
RUN wget -q -O - https://dl.google.com/linux/linux_signing_key.pub | apt-key add -
RUN apt-add-repository ppa:swi-prolog/stable
RUN apt-get update
RUN apt-get install -y swi-prolog build-essential git-all

RUN chown -hR tester /home/tester/

COPY capstone.conf /etc/ld.so.conf.d/
RUN ldconfig -v

COPY ideaIC-2022.3.3.tar.gz /home/tester/

RUN tar -xzf /home/tester/ideaIC-*.tar.gz -C /opt

USER tester

ENV HOME=/home/tester
ENV PATH=$PATH:/home/tester/.cargo/bin

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

COPY intellij-rust-0.4.188.SNAPSHOT-223-dev.zip /home/tester

# components needed for rustup to install the backend
RUN rustup toolchain install nightly-2023-01-23-x86_64-unknown-linux-gnu
RUN rustup default nightly-2023-01-23-x86_64-unknown-linux-gnu

RUN rustc --version > /dev/null #make sure rust is installed, install from https://www.rust-lang.org/tools/install
RUN swipl --version > /dev/null #make sure swipl is installed, install from https://www.swi-prolog.org/build/unix.html

RUN rustup component add rust-src rustc-dev llvm-tools-preview

# install the extracting backends
RUN cargo install rem-controller rem-borrower rem-repairer

RUN ln -s /opt/idea-IC-223.8836.41/bin/idea.sh /home/tester/Desktop/

COPY sample_projects.tar /home/tester/Desktop/sample_projects.tar
RUN tar -xzf /home/tester/Desktop/sample_projects.tar --directory /home/tester/Desktop/
RUN mv /home/tester/Desktop/artefact_sample_projects /home/tester/Desktop/sample_projects

COPY README.md /home/tester/Desktop/README.md

COPY JetBrains.zip /home/tester/

USER root
RUN apt-get install -y zip

USER tester
RUN mkdir -p /home/tester/.local/share/
RUN unzip /home/tester/JetBrains.zip -d /home/tester/.local/share/

USER root
RUN echo "root:root" | chpasswd
RUN ldconfig -v
