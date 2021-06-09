FROM rust:1.52.1

# 1. setup ruby
RUN git clone https://github.com/rbenv/rbenv.git /root/.rbenv
RUN git clone https://github.com/rbenv/ruby-build.git /root/.rbenv/plugins/ruby-build
RUN /root/.rbenv/plugins/ruby-build/install.sh
ENV PATH /root/.rbenv/bin:/root/.rbenv/shims:$PATH
RUN echo 'eval "$(rbenv init -)"' >> /etc/profile.d/rbenv.sh      
RUN echo 'eval "$(rbenv init -)"' >> .bashrc

# 2. setup ruby for dependabot
RUN rbenv install 2.6.6
RUN rbenv global 2.6.6
RUN gem install bundler

# 3. setup backend
WORKDIR /app
COPY . .

# 4. setup rustup according to rust-toolchain file
RUN rustup update

# 5. init dependabot
RUN cd metrics/dependabot && bundle install && cd - 

# 6. configure and run
WORKDIR /app/ui
EXPOSE 8081
ENV RUST_LOG=info
ENV ROCKET_ADDRESS="0.0.0.0"
ENV ROCKET_PORT=8081
CMD ["cargo", "run", "--bin", "web-ui"]
