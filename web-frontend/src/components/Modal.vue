<template>
  <transition name="modal">
    <div class="modal" tabindex="-1" v-if="show">
      <div class="modal-dialog">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">{{ name }}</h5>
            <button
              type="button"
              class="btn-close"
              data-bs-dismiss="modal"
              aria-label="Close"
              @click="close"
            ></button>
          </div>
          <div class="modal-body">
            <p>
              To create a PR easily, make sure you are on an up-to-date branch
              of the
              <code>main</code> branch and paste the following in your terminal:
            </p>
            <pre><code>cargo update -p {{ name }}:{{ version}} --precise {{ new_version }}</code></pre>
          </div>
          <div class="modal-footer">
            <button
              type="button"
              class="btn btn-secondary"
              data-bs-dismiss="modal"
              @click="close"
            >
              Close
            </button>
            <button type="button" class="btn btn-primary">Save changes</button>
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>

<script>
export default {
  name: "Modal",
  data() {
    return {
      show: false,
      name: "",
      version: "",
      new_version: "",
    };
  },
  methods: {
    close() {
      this.show = false;
    },
    open(name, version, new_version) {
      this.name = name;
      this.version = version;
      this.new_version = new_version;
      this.show = true;
    },
  },
};
</script>

<style scoped>
.modal {
  display: block !important;
}
</style>