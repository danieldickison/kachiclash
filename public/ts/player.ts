const profileSection = document.getElementById('profile')
const toggleEditMode = (event: Event) => {
  event.preventDefault()
  profileSection.classList.toggle('editing')
}
for (const edit of profileSection.querySelectorAll('.buttons > .edit')) {
  edit.addEventListener('click', toggleEditMode)
}
for (const cancel of profileSection.querySelectorAll(':scope > form .cancel')) {
  cancel.addEventListener('click', toggleEditMode)
}

export default {}
