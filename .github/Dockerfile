FROM denoland/deno:ubuntu

ENV DEBIAN_FRONTEND=noninteractive

ENV LANG=en_US.UTF-8

RUN apt update

RUN apt install -y locales && locale-gen --purge en_US.UTF-8

RUN apt install -y curl jq tmux vim git

WORKDIR /data

COPY ./bin/minix /usr/bin/minix

RUN chmod +x /usr/bin/minix

EXPOSE 30333

ENTRYPOINT ["minix"]
