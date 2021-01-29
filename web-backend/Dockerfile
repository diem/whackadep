FROM rust:1.49.0

# setup rust
RUN rustup default nightly

# setup ruby
RUN git clone https://github.com/rbenv/rbenv.git /root/.rbenv
RUN git clone https://github.com/rbenv/ruby-build.git /root/.rbenv/plugins/ruby-build
RUN /root/.rbenv/plugins/ruby-build/install.sh
ENV PATH /root/.rbenv/bin:/root/.rbenv/shims:$PATH
RUN echo 'eval "$(rbenv init -)"' >> /etc/profile.d/rbenv.sh      
RUN echo 'eval "$(rbenv init -)"' >> .bashrc

# setup dependabot
RUN rbenv install 2.6.6
RUN rbenv global 2.6.6

# setup backend
WORKDIR /app
COPY . .
RUN cd dependabot && gem install bundler && bundle install && cd - 

#
ENV RUST_LOG=info
CMD ["cargo", "run"]
