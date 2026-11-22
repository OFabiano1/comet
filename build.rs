// Ícone do .exe (cometa roxo): aparece no explorador e na barra de tarefas.
fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/comet.ico");
        res.compile().expect("falha ao embutir o ícone");
    }
}
