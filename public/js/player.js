(function () {
'use strict';

const profileSection = document.getElementById('profile');
const toggleEditMode = (event) => {
    event.preventDefault();
    profileSection.classList.toggle('editing');
}
for (const edit of profileSection.querySelectorAll(':scope > .edit')) {
    edit.addEventListener('click', toggleEditMode);
}
for (const cancel of profileSection.querySelectorAll(':scope > form .cancel')) {
    cancel.addEventListener('click', toggleEditMode);
}

})();
