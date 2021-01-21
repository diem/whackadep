FROM rust:1-buster

# config
ENV REPO "https://github.com/diem/diem.git"
ENV REPO_DIR diem
RUN printenv

# Setup cron
RUN apt-get update && apt-get -y install cron
COPY cronfile /etc/cron.d/cronfile
RUN chmod 0644 /etc/cron.d/cronfile
RUN crontab /etc/cron.d/cronfile
RUN touch /var/log/cron.log

# Run the command on container startup
CMD cron && tail -f /var/log/cron.log
