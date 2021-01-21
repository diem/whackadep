FROM rust:1-buster

# config
ENV REPO "https://github.com/diem/diem.git"
ENV REPO_DIR diem
RUN printenv

# get guppy
RUN cargo install --git https://github.com/facebookincubator/cargo-guppy --rev ce039c81deaa22d4bae9e4a7e4bed8448acfde9e

# do any initialization setup
WORKDIR /app
COPY init/init.sh /app
RUN chmod +x init.sh
RUN /app/init.sh

# Setup cron
RUN apt-get update && apt-get -y install cron
COPY cronfile /etc/cron.d/cronfile
RUN chmod 0644 /etc/cron.d/cronfile
RUN crontab /etc/cron.d/cronfile
RUN touch /var/log/cron.log

# Run the command on container startup
CMD cron && tail -f /var/log/cron.log
