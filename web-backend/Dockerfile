FROM rust:1.50.0

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

# 3. setup rust for easy caching
ENV CARGO_HOME=/cargo

# 4. setup backend
WORKDIR /app
COPY . .

# 5. setup rust
# RUN rustup update # can't run rustup with this base image

# 6. init dependabot
RUN cd metrics/dependabot && bundle install && cd - 

# Note: we could init cargo audit and cargo tree here, 
# but we do it at the application level instead.

# 7. configure and run
EXPOSE 8081
ENV RUST_LOG=info
ENV ROCKET_ADDRESS="0.0.0.0"
ENV ROCKET_PORT=8081
CMD ["cargo", "run"]
